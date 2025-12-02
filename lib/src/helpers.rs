use crate::errors::{Error, Result};

use serde_yaml::Value as YamlValue;
use serde_yaml::value::TaggedValue as YamlTaggedValue;
use serde_yaml_ng as serde_yaml;

use std::fs;
use std::path::Path;

pub fn load_yaml(file_path: impl AsRef<Path>) -> Result<YamlValue> {
    let file_reader = fs::File::open(file_path)?;
    Ok(serde_yaml::from_reader(file_reader)?)
}

pub fn tagged_value_as_str(tagged_value: &YamlTaggedValue) -> Result<&str> {
    tagged_value
        .value
        .as_str()
        .ok_or_else(|| Error::InvalidStringValue(format!("{:?}", tagged_value.value)))
}

#[cfg(feature = "include_bin")]
pub fn load_as_base64(normalized_file_path: impl AsRef<Path>) -> Result<String> {
    use base64::{Engine, engine::general_purpose as b64_engine};

    let bytes = fs::read(normalized_file_path)?;
    let mut data = String::new();
    b64_engine::STANDARD.encode_string(bytes, &mut data);

    Ok(data)
}

#[cfg(feature = "glob")]
pub fn merge_yaml_values_in_place(a: &mut YamlValue, b: YamlValue) -> Result<()> {
    match (a, b) {
        (YamlValue::Mapping(a_map), YamlValue::Mapping(b_map)) => {
            for (k, b_v) in b_map {
                if let Some(a_v) = a_map.get_mut(&k) {
                    merge_yaml_values_in_place(a_v, b_v)?;
                } else {
                    a_map.insert(k, b_v);
                }
            }
            Ok(())
        }
        (YamlValue::Sequence(a_seq), YamlValue::Sequence(b_seq)) => {
            a_seq.reserve(b_seq.len());
            a_seq.extend(b_seq);
            Ok(())
        }
        _ => Err(Error::MergeError()),
    }
}
