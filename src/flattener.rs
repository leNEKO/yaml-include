use serde_yaml::{
    value::{Tag, TaggedValue},
    Mapping, Value,
};

use std::{fmt, fs::read_to_string, path::PathBuf};

use crate::helpers::{load_as_base64, load_yaml};

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

    fn parse(self) -> Value {
        let file_path = self.root_path.clone();
        let input = load_yaml(file_path).unwrap();

        self.recursive_flatten(input)
    }

    fn recursive_flatten(self, input: Value) -> Value {
        match input {
            Value::Sequence(seq) => seq
                .iter()
                .map(|v| self.clone().recursive_flatten(v.clone()))
                .collect(),
            Value::Mapping(map) => Value::Mapping(Mapping::from_iter(
                map.iter()
                    .map(|(k, v)| (k.clone(), self.clone().recursive_flatten(v.clone()))),
            )),
            Value::Tagged(tagged_value) => match tagged_value.tag.to_string().as_str() {
                "!include" => {
                    let value = tagged_value.value.as_str().unwrap();
                    let file_path = PathBuf::from(value);

                    self.handle_include_extension(file_path)
                }
                _ => Value::Tagged(tagged_value),
            },
            // default no transform
            _ => input,
        }
    }

    fn circular_reference_guard(&self, file_path: &PathBuf) -> bool {
        self.seen_paths.contains(file_path)
    }

    fn handle_include_extension(&self, file_path: PathBuf) -> Value {
        let normalized_file_path = self.process_path(&file_path);

        let result = match normalized_file_path.extension() {
            Some(os_str) => match os_str.to_str() {
                Some("yaml") | Some("yml") => {
                    if self.circular_reference_guard(&normalized_file_path) {
                        let path_string = &file_path.display();

                        return Value::Tagged(
                            TaggedValue {
                                tag: Tag::new("circular"),
                                value: Value::String(path_string.to_string()),
                            }
                            .into(),
                        );
                    }
                    let mut seen_paths = self.seen_paths.clone();
                    seen_paths.push(normalized_file_path.clone());

                    Flattener::new(normalized_file_path, seen_paths).parse()
                }
                // inlining markdow and text files
                Some("txt") | Some("markdown") | Some("md") => {
                    Value::String(read_to_string(normalized_file_path).unwrap())
                }
                // inlining other include as binary files
                None | Some(&_) => Value::Tagged(Box::new(TaggedValue {
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
            },
            _ => panic!("{:?} path missing file extension", normalized_file_path),
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

        joined
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
fn test_flattener() {
    let expected = read_to_string("data/expected.yml").unwrap();
    let flattener = Flattener::new(PathBuf::from("data/root.yml"), vec![]);
    let actual = flattener.to_string();

    assert_eq!(expected, actual);
}
