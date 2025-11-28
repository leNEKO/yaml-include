use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{env, fmt, fs};

use serde_yaml::value::{Tag, TaggedValue};
use serde_yaml::{Mapping, Value};
use serde_yaml_ng as serde_yaml;

#[cfg(feature = "glob")]
use crate::glob::Glob;

use crate::errors::{Error, Result};
use crate::helpers::{load_yaml, tagged_value_as_str};

#[cfg(feature = "glob")]
use crate::helpers::merge_yaml_values_in_place;

#[cfg(feature = "include_bin")]
use crate::helpers::load_as_base64;

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
    /// use yaml_include::Transformer;
    ///
    /// if let Ok(transformer) = Transformer::new("../examples/sample/main.yml", false) {
    ///     dbg!(transformer);
    /// };
    /// ```
    pub fn new(root_path: impl AsRef<Path>, strict: bool) -> Result<Self> {
        Self::new_node(root_path.as_ref().to_path_buf(), strict, HashSet::new())
    }

    /// Parse yaml and output a serde_yaml::Value
    ///
    /// # Example:
    ///
    /// ```
    /// use yaml_include::Transformer;
    ///
    /// let transformer = Transformer::new("../examples/sample/main.yml", false).unwrap();
    /// let parsed = transformer.parse().unwrap();
    /// dbg!(parsed);
    /// ```
    pub fn parse(&self) -> Result<Value> {
        let input = load_yaml(&self.root_path)?;
        self.recursive_process(Ok(input))
    }

    /// Parse yaml and output a string
    ///
    /// # Example:
    ///
    /// ```
    /// use yaml_include::Transformer;
    ///
    /// let transformer = Transformer::new("../examples/sample/main.yml", false).unwrap();
    /// let parsed = transformer.parse_to_string();
    /// dbg!(parsed);
    /// ```
    pub fn parse_to_string(&self) -> Result<String> {
        let input = load_yaml(&self.root_path)?;
        let res = self.recursive_process(Ok(input))?;
        serde_yaml::to_string(&res).map_err(|e| e.into())
    }

    fn new_node(root_path: PathBuf, strict: bool, seen_paths: HashSet<PathBuf>) -> Result<Self> {
        let normalized_path = fs::canonicalize(&root_path)?;

        // Circular reference guard
        if seen_paths.contains(&normalized_path) {
            return Err(Error::CircularReference(normalized_path));
        }

        let mut new_seen_paths = seen_paths;
        new_seen_paths.insert(normalized_path.clone());
        Ok(Transformer {
            error_on_circular: strict,
            root_path: normalized_path,
            seen_paths: new_seen_paths,
        })
    }

    fn recursive_process(&self, input: Result<Value>) -> Result<Value> {
        match input {
            Ok(Value::Sequence(seq)) => seq
                .iter()
                .map(|v| self.recursive_process(Ok(v.clone())))
                .collect(),
            Ok(Value::Mapping(map)) => {
                let new_map: Mapping = map
                    .iter()
                    .map(|(k, v)| {
                        let new_v = self.recursive_process(Ok(v.clone()))?;
                        Ok((k.clone(), new_v))
                    })
                    .collect::<Result<Mapping>>()?;
                Ok(Value::Mapping(new_map))
            }
            Ok(Value::Tagged(tagged)) => {
                match tagged.tag.to_string().as_str() {
                    "!env" => self.include_env(tagged_value_as_str(&tagged)?),
                    "!include" => self.include_pattern_from_ext(tagged_value_as_str(&tagged)?),
                    "!include_yaml" | "!include_yml" => {
                        self.include_yaml_pattern(tagged_value_as_str(&tagged)?)
                    }
                    "!include_text" | "!include_txt" | "!file" => {
                        self.include_text_pattern(tagged_value_as_str(&tagged)?)
                    }

                    #[cfg(feature = "include_bin")]
                    "!include_bin" | "!bin" => {
                        self.include_bin_pattern(tagged_value_as_str(&tagged)?)
                    }

                    // Unknown tag, keep it
                    _ => Ok(Value::Tagged(tagged)),
                }
            }
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }

    fn include_pattern_from_ext(&self, tagged_value: &str) -> Result<Value> {
        self.include_pattern(tagged_value, |path| {
            self.include_file_from_ext(path, tagged_value)
        })
    }

    fn include_file_from_ext(&self, normalized_path: &Path, tagged_value: &str) -> Result<Value> {
        let ext = normalized_path.extension().and_then(|s| s.to_str());
        match ext {
            Some("yaml") | Some("yml") | Some("json") => {
                self.include_yaml_file(normalized_path, tagged_value)
            }
            Some("txt") | Some("markdown") | Some("md") => Self::include_text_file(normalized_path),
            _ => Self::include_unknown_file(normalized_path),
        }
    }

    #[cfg(not(feature = "include_bin"))]
    fn include_unknown_file(normalized_path: &Path) -> Result<Value> {
        Self::include_text_file(normalized_path)
    }

    #[cfg(feature = "include_bin")]
    fn include_unknown_file(normalized_path: &Path) -> Result<Value> {
        Self::include_bin_file(normalized_path)
    }

    fn include_text_pattern(&self, tagged_value: &str) -> Result<Value> {
        self.include_pattern(tagged_value, |normalized_path| {
            Self::include_text_file(normalized_path)
        })
    }

    fn include_text_file(normalized_path: &Path) -> Result<Value> {
        Ok(Value::String(fs::read_to_string(normalized_path)?))
    }

    #[cfg(feature = "include_bin")]
    fn include_bin_pattern(&self, tagged_value: &str) -> Result<Value> {
        self.include_pattern(tagged_value, |normalized_path| {
            Self::include_bin_file(normalized_path)
        })
    }

    #[cfg(feature = "include_bin")]
    fn include_bin_file(normalized_path: &Path) -> Result<Value> {
        Ok(Value::Tagged(Box::new(TaggedValue {
            tag: Tag::new("binary"),
            value: Value::Mapping(Mapping::from_iter([
                (
                    Value::String("filename".into()),
                    Value::String(
                        normalized_path
                            .file_name()
                            .ok_or_else(|| {
                                Error::IncludeError(
                                    normalized_path.to_path_buf(),
                                    "Unable to include".into(),
                                )
                            })?
                            .to_string_lossy()
                            .to_string(),
                    ),
                ),
                (
                    Value::String("base64".into()),
                    Value::String(load_as_base64(normalized_path)?),
                ),
            ])),
        })))
    }

    fn include_yaml_pattern(&self, tag_value: &str) -> Result<Value> {
        self.include_pattern(tag_value, |normalized_path| {
            self.include_yaml_file(normalized_path, tag_value)
        })
    }

    fn include_yaml_file(&self, file_path: &Path, tag_value: &str) -> Result<Value> {
        let res = Transformer::new_node(
            file_path.to_path_buf(),
            self.error_on_circular,
            self.seen_paths.clone(),
        );
        match res {
            Ok(child) => child.parse(),
            Err(Error::CircularReference(_)) if !self.error_on_circular => Ok(Value::Tagged(
                TaggedValue {
                    tag: Tag::new("circular"),
                    value: Value::String(tag_value.to_owned()),
                }
                .into(),
            )),
            Err(err) => Err(err),
        }
    }

    fn include_env(&self, env_var_name: &str) -> Result<Value> {
        match env::var(env_var_name) {
            Ok(s) => Ok(Value::String(s)),
            _ => Ok(Value::Null),
        }
    }

    fn process_path(&self, file_path: impl AsRef<Path>) -> Result<PathBuf> {
        let mut current_file_path = file_path.as_ref().to_path_buf();
        if !current_file_path.is_absolute() {
            current_file_path = self
                .root_path
                .parent()
                .ok_or_else(|| {
                    Error::IncludeError(
                        self.root_path.clone(),
                        "Unable to get parent folder".into(),
                    )
                })?
                .join(file_path);
        }
        match current_file_path.is_file() {
            true => Ok(fs::canonicalize(current_file_path)?),
            false => Err(Error::IncludeError(
                current_file_path.clone(),
                "Not a file".into(),
            )),
        }
    }

    #[cfg(feature = "glob")]
    fn include_pattern<F>(&self, tagged_value: &str, include_callback: F) -> Result<Value>
    where
        F: Fn(&Path) -> Result<Value>,
    {
        let root_dir = self
            .root_path
            .parent()
            .ok_or_else(|| Error::NoParentError(self.root_path.clone()))?;
        let glob = Glob::new(tagged_value, root_dir)?;
        if glob.is_fixed() {
            return self.include_single_file(tagged_value, include_callback);
        }

        let mut merged_dict = Value::Mapping(Mapping::new());
        for entry_res in glob.into_iter() {
            let entry = entry_res?;

            if entry.full_path == self.root_path {
                continue; // Ignore the current yaml file to allow *.yaml in the same folder
            }

            let value = include_callback(&entry.full_path)?;

            let base_filename = entry
                .full_path
                .file_stem()
                .and_then(|c| c.to_str())
                .ok_or_else(|| {
                    Error::IncludeError(entry.full_path.clone(), "Bad filename".into())
                })?;

            let mut data = Mapping::new();
            data.insert(Value::String(base_filename.to_owned()), value);

            let wrapped = Value::Mapping(data);
            merge_yaml_values_in_place(&mut merged_dict, wrapped)?;
        }
        Ok(merged_dict)
    }

    #[cfg(not(feature = "glob"))]
    fn include_pattern<F>(&self, tagged_value: &str, include_callback: F) -> Result<Value>
    where
        F: Fn(&Path) -> Result<Value>,
    {
        include_single_file(tagged_value, include_callback)
    }

    fn include_single_file<F>(&self, tagged_value: &str, include_callback: F) -> Result<Value>
    where
        F: Fn(&Path) -> Result<Value>,
    {
        let normalized_path = self.process_path(tagged_value)?;
        include_callback(&normalized_path)
    }
}

impl fmt::Display for Transformer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.parse_to_string() {
            Ok(serialized) => write!(f, "{}", serialized),
            Err(err) => write!(f, "{}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_transformer_deep_include() -> Result<()> {
        run_test_example("deep_include")
    }

    #[test]
    fn test_transformer_sample() -> Result<()> {
        run_test_example("sample")
    }

    #[test]
    fn test_transformer_glob() -> Result<()> {
        run_test_example("glob")
    }

    #[test]
    fn test_transformer_glob_ignored_local_yaml() -> Result<()> {
        run_test_example("glob_ignored_local_yaml")
    }

    #[test]
    fn test_transformer_circular_error() {
        let worspace_dir = fs::canonicalize(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
        let test_file = worspace_dir.join(format!("examples/circular_error/main.yml"));
        if !&test_file.exists() {
            panic!("Unable to find file test example file {:?}", &test_file);
        }

        let transformer = Transformer::new(&test_file, true).expect("Unable to create transformer");
        let circular_ref_error = match transformer.parse() {
            Err(Error::CircularReference(ref_file)) => Some(ref_file),
            Err(err) => panic!("An unexpected kind of error detected: {err:?}"),
            Ok(content) => panic!("An unexpected result detected: {content:?}"),
        };
        assert_eq!(circular_ref_error, Some(test_file));
    }

    fn run_test_example(example_name: &str) -> Result<()> {
        let worspace_dir = fs::canonicalize(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
        let expected_filepath =
            worspace_dir.join(format!("examples/_expected_outputs/{example_name}.yml"));
        let expected = fs::read_to_string(&expected_filepath).expect(
            format!(
                "Unable to locate expected result file {:?}",
                &expected_filepath
            )
            .as_str(),
        );

        let test_file = worspace_dir.join(format!("examples/{example_name}/main.yml"));
        if !test_file.exists() {
            panic!("Unable to find file test example file {:?}", test_file);
        }

        let transformer = Transformer::new(test_file, false);
        let actual = transformer
            .expect("Unable to create transformer")
            .to_string();
        assert_eq!(expected, actual);

        Ok(())
    }
}
