use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use serde_yaml::Value as YamlValue;

use std::{
    fs::{read, File},
    path::PathBuf,
};

pub fn load_yaml(file_path: PathBuf) -> Result<YamlValue> {
    let file_reader = File::open(file_path).expect("Unable to open file");
    let data: YamlValue = serde_yaml::from_reader(file_reader)?;

    Ok(data)
}

pub fn load_as_base64(normalized_file_path: &PathBuf) -> Result<String> {
    let bytes = read(normalized_file_path).expect("Unable to open file");
    let mut data = String::new();
    general_purpose::STANDARD.encode_string(bytes, &mut data);

    Ok(data)
}
