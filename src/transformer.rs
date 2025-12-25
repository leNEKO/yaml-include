use anyhow::{anyhow, Result};
use serde_yaml_ng::{
    value::{Tag, TaggedValue},
    Mapping, Value,
};

use std::{
    collections::HashSet,
    fmt,
    fs::{canonicalize, read_to_string},
    path::PathBuf,
    str::FromStr,
};

use crate::helpers::{load_as_base64, load_yaml};

struct FilePath {
    path: PathBuf,
    extension: Extension,
}

enum Extension {
    Yaml,
    Text,
    Binary,
}

#[derive(Debug)]
enum ParseError {
    MissingPath,
    MissingExtension,
}

impl FromStr for Extension {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "yaml" | "yml" | "json" => Ok(Self::Yaml),
            "md" | "markdown" | "txt" => Ok(Self::Text),
            _ => Ok(Self::Binary),
        }
    }
}

impl TryFrom<Mapping> for FilePath {
    type Error = ParseError;

    fn try_from(value: Mapping) -> Result<Self, Self::Error> {
        let path = value
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or(ParseError::MissingPath)?
            .into();

        let extension = Extension::from_str(
            value
                .get("extension")
                .and_then(|value| value.as_str())
                .ok_or(ParseError::MissingExtension)?,
        )
        .expect("Infaillible conversion");

        Ok(Self { path, extension })
    }
}

impl TryFrom<String> for FilePath {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let path: PathBuf = value.into();

        let extension = Extension::from_str(
            path.extension()
                .and_then(|ext| ext.to_str())
                .ok_or(ParseError::MissingExtension)?,
        )
        .expect("Infaillible conversion");

        Ok(Self { path, extension })
    }
}

/// Processing yaml with include documents through `!include <path>` tag.
///
/// ## Features
///
/// - include and parse recursively `yaml` (and `json`) files
/// - include `markdown` and `txt` text files
/// - include other types as `base64` encoded binary data.
/// - optionaly handle gracefully circular references with `!circular` tag
///
/// ## Example
/// ```
/// use std::path::PathBuf;
/// use yaml_include::Transformer;
///
/// let path = PathBuf::from("data/sample/main.yml");
/// if let Ok(transformer) = Transformer::new(path, false) {
///     println!("{}", transformer);
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Transformer {
    error_on_circular: bool,
    root_path: PathBuf,
    seen_paths: HashSet<PathBuf>, // for circular reference detection
}

impl Transformer {
    /// Instance a transformer from a yaml file path.
    ///
    /// # Example:
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use yaml_include::Transformer;
    ///
    /// let path = PathBuf::from("data/sample/main.yml");
    /// if let Ok(transformer) = Transformer::new(path, false) {
    ///     dbg!(transformer);
    /// };
    /// ```
    pub fn new(root_path: PathBuf, strict: bool) -> Result<Self> {
        Self::new_node(root_path, strict, None)
    }

    /// Parse yaml with recursively processing `!include`
    ///
    /// # Example:
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use yaml_include::Transformer;
    ///
    /// let path = PathBuf::from("data/sample/main.yml");
    /// if let Ok(transformer) = Transformer::new(path, false) {
    ///     let parsed = transformer.parse();
    ///     dbg!(parsed);
    /// };
    /// ```
    pub fn parse(&self) -> Value {
        let file_path = self.root_path.clone();
        let input = load_yaml(file_path).unwrap();

        self.clone().recursive_process(input)
    }

    fn new_node(
        root_path: PathBuf,
        strict: bool,
        seen_paths_option: Option<HashSet<PathBuf>>,
    ) -> Result<Self> {
        let mut seen_paths = seen_paths_option.unwrap_or_default();

        let normalized_path = canonicalize(&root_path).unwrap();

        // Circular reference guard
        if seen_paths.contains(&normalized_path) {
            return Err(anyhow!(
                "circular reference: {}",
                &normalized_path.display()
            ));
        }

        seen_paths.insert(normalized_path);

        Ok(Transformer {
            error_on_circular: strict,
            root_path,
            seen_paths,
        })
    }

    fn recursive_process(self, input: Value) -> Value {
        match input {
            Value::Sequence(seq) => seq
                .iter()
                .map(|v| self.clone().recursive_process(v.clone()))
                .collect(),
            Value::Mapping(map) => Value::Mapping(Mapping::from_iter(
                map.iter()
                    .map(|(k, v)| (k.clone(), self.clone().recursive_process(v.clone()))),
            )),
            Value::Tagged(tagged_value) => match tagged_value.tag.to_string().as_str() {
                "!include" => {
                    let file_path: FilePath = match tagged_value.value {
                        Value::String(path) => path.try_into().unwrap(),
                        Value::Mapping(mapping) => mapping.try_into().unwrap(),
                        _ => panic!("Unsupported Value"),
                    };

                    self.handle_include_extension(file_path)
                }
                _ => Value::Tagged(tagged_value),
            },
            // default no transform
            _ => input,
        }
    }

    fn handle_include_extension(&self, file_path: FilePath) -> Value {
        let normalized_file_path = self.process_path(&file_path.path);

        let result = match file_path.extension {
            Extension::Yaml => {
                match Transformer::new_node(
                    normalized_file_path,
                    self.error_on_circular,
                    Some(self.seen_paths.clone()),
                ) {
                    Ok(transformer) => transformer.parse(),
                    Err(e) => {
                        if self.error_on_circular {
                            panic!("{:?}", e);
                        }

                        return Value::Tagged(
                            TaggedValue {
                                tag: Tag::new("circular"),
                                value: Value::String(file_path.path.display().to_string()),
                            }
                            .into(),
                        );
                    }
                }
            }
            // inlining markdow and text files
            Extension::Text => Value::String(read_to_string(normalized_file_path).unwrap()),
            // inlining other include as binary files
            Extension::Binary => Value::Tagged(Box::new(TaggedValue {
                tag: Tag::new("binary"),
                value: Value::Mapping(Mapping::from_iter([
                    (
                        Value::String("filename".into()),
                        Value::String(
                            normalized_file_path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                        ),
                    ),
                    (
                        Value::String("base64".into()),
                        Value::String(load_as_base64(&normalized_file_path).unwrap()),
                    ),
                ])),
            })),
        };

        result
    }

    fn process_path(&self, file_path: &PathBuf) -> PathBuf {
        if file_path.is_absolute() {
            return file_path.clone();
        }
        let joined = self.root_path.parent().unwrap().join(file_path);

        if !joined.is_file() {
            panic!("{:?} not found", joined);
        }

        canonicalize(joined).unwrap()
    }
}

impl fmt::Display for Transformer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_yaml_ng::to_string(&self.clone().parse()).unwrap()
        )
    }
}

#[test]
fn test_transformer() -> Result<()> {
    let expected = read_to_string("data/expected.yml").unwrap();
    let transformer = Transformer::new(PathBuf::from("data/root.yml"), false);
    let actual = transformer?.to_string();

    assert_eq!(expected, actual);

    Ok(())
}
