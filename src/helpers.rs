use anyhow::Result;
use regex::Regex;
use serde_yaml::Value as YamlValue;

use std::{fs::File, path::PathBuf};

pub fn load_yaml(file_path: PathBuf) -> Result<YamlValue> {
    let file_reader = File::open(file_path).expect("Unable to open file");
    let data: YamlValue = serde_yaml::from_reader(file_reader)?;

    Ok(data)
}

pub fn is_dirty_include(text: String) -> Option<PathBuf> {
    Regex::new(r"\$\{(?P<file_path>.+)\}")
        .unwrap()
        .captures(text.as_str())
        .map(|v| v.name("file_path").unwrap().as_str().into())
}
#[test]
fn test_is_include() {
    let actual = is_dirty_include("${/hello.world/test.yaml}".into());
    let expected = Some("/hello.world/test.yaml".into());

    dbg!(&actual, &expected);

    assert_eq!(actual, expected);
}
#[test]
fn test_is_not_include() {
    let actual = is_dirty_include("/hello.world/test.yaml".into());
    let expected = None;

    assert_eq!(actual, expected);
}
