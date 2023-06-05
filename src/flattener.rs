use serde_yaml::{
    value::{Tag, TaggedValue},
    Mapping, Value,
};

use std::{
    fmt,
    fs::{canonicalize, read, read_to_string},
    path::PathBuf,
};

use crate::helpers::{is_dirty_include, load_yaml};

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
        let result = match input {
            Value::Sequence(seq) => seq
                .iter()
                .map(|v| self.clone().recursive_flatten(v.clone()))
                .collect(),
            Value::Bool(v) => Value::Bool(v),
            Value::Null => Value::Null,
            Value::Number(v) => Value::Number(v),
            Value::String(data) => match is_dirty_include(data.clone()) {
                None => Value::String(data),
                Some(file_path) => self.handle_include_extension(file_path),
            },
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
        };

        result
    }

    fn circular_reference_guard(&self, file_path: &PathBuf) -> bool {
        self.seen_paths.contains(file_path)
    }

    fn handle_include_extension(&self, file_path: PathBuf) -> Value {
        let normalized_file_path = self.process_path(file_path);

        let result = match normalized_file_path
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .to_lowercase()
            .as_str()
        {
            "yaml" | "yml" => {
                if self.circular_reference_guard(&normalized_file_path) {
                    let path_string = &normalized_file_path.display();

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
            "txt" | "markdown" | "md" => {
                Value::String(read_to_string(normalized_file_path).unwrap())
            }
            _ => {
                let data = read(normalized_file_path).unwrap();

                Value::Tagged(Box::new(TaggedValue {
                    tag: Tag::new("binary"),
                    value: Value::String(base64::encode(data)),
                }))
            }
        };

        result
    }

    fn process_path(&self, file_path: PathBuf) -> PathBuf {
        if file_path.is_absolute() {
            return file_path;
        }
        let joined = self.root_path.parent().unwrap().join(file_path);

        canonicalize(joined).unwrap()
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
