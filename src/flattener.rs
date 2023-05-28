use std::{fmt, fs::File, path::PathBuf};

use anyhow::Result;
use regex::Regex;
use serde_yaml::{
    value::{Tag, TaggedValue},
    Mapping, Value as YamlValue,
};

#[derive(Debug, Clone)]
pub struct Flattener {
    root_path: PathBuf,
    seen_paths: Vec<PathBuf>, // for circular reference detection
}

impl Flattener {
    pub fn new(root_path: PathBuf, mut seen_paths: Vec<PathBuf>) -> Self {
        seen_paths.push(root_path.clone());

        Flattener {
            root_path,
            seen_paths,
        }
    }

    fn parse(self) -> YamlValue {
        let file_path = self.root_path.clone();
        let input = load_yaml(file_path).unwrap();
        self.recursive_flatten(input)
    }

    fn recursive_flatten(self, input: YamlValue) -> YamlValue {
        return match input {
            YamlValue::Sequence(seq) => seq
                .iter()
                .map(|v| self.clone().recursive_flatten(v.clone()))
                .collect(),
            YamlValue::Bool(v) => YamlValue::Bool(v),
            YamlValue::Null => YamlValue::Null,
            YamlValue::Number(v) => YamlValue::Number(v),
            YamlValue::String(data) => match is_include(data.clone()) {
                None => YamlValue::String(data),
                Some(file_path) => self.handle_include_extension(file_path, data),
            },
            YamlValue::Mapping(map) => YamlValue::Mapping(Mapping::from_iter(
                map.iter()
                    .map(|(k, v)| (k.clone(), self.clone().recursive_flatten(v.clone()))),
            )),
            YamlValue::Tagged(tagged) => YamlValue::Tagged(tagged), // TODO process !!inc ?
        };
    }

    fn circular_reference_guard(&self, file_path: &PathBuf) -> bool {
        self.seen_paths.contains(file_path)
    }

    fn handle_include_extension(&self, file_path: PathBuf, data: String) -> YamlValue {
        match file_path.extension().unwrap().to_str().unwrap() {
            "yaml" | "yml" => {
                let normalized_file_path = self.process_path(file_path);
                if self.circular_reference_guard(&normalized_file_path) {
                    let path_string = &normalized_file_path.display();
                    return YamlValue::Tagged(
                        TaggedValue {
                            tag: Tag::new("circular"),
                            value: YamlValue::String(path_string.to_string()),
                        }
                        .into(),
                    );
                }
                let mut seen_paths = self.seen_paths.clone();
                seen_paths.push(normalized_file_path.clone());

                Flattener::new(normalized_file_path, seen_paths).parse()
            }
            _ => YamlValue::String(data),
        }
    }

    fn process_path(&self, file_path: PathBuf) -> PathBuf {
        if file_path.is_absolute() {
            return file_path;
        }

        self.root_path.parent().unwrap().join(file_path)
    }
}

impl fmt::Display for Flattener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_yaml::to_string(&self.clone().parse()).unwrap()
        )
    }
}
#[test]
fn test_create_flattener() {
    let t = Flattener::new("data/circular/a.yml".into(), vec![]);
    let r = t.parse();
    dbg!(r);
}

fn load_yaml(file_path: PathBuf) -> Result<YamlValue> {
    let file_reader = File::open(file_path).expect("Unable to open file");
    let data: YamlValue = serde_yaml::from_reader(file_reader)?;

    Ok(data)
}

fn is_include(text: String) -> Option<PathBuf> {
    Regex::new(r"\$\{(?P<file_path>.+)\}")
        .unwrap()
        .captures(text.as_str())
        .map(|v| v.name("file_path").unwrap().as_str().into())
}
#[test]
fn test_is_include() {
    let actual = is_include("${/hello.world/test.yaml}".into());
    let expected = Some("/hello.world/test.yaml".into());

    dbg!(&actual, &expected);

    assert_eq!(actual, expected);
}
#[test]
fn test_is_not_include() {
    let actual = is_include("/hello.world/test.yaml".into());
    let expected = None;

    assert_eq!(actual, expected);
}
