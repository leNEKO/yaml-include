use crate::errors;
use crate::transformer::Transformer;

use std::path::Path;

use serde_yaml_ng as serde_yaml;

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
/// use yaml_include;
///
/// let path = "examples/sample/main.yml";
/// if let Ok(res) = yaml_include::read(&path) {
///     println!("{:?}", res);
/// };
/// ```
pub fn read(file_path: impl AsRef<Path>) -> errors::Result<serde_yaml::Value> {
    let transformer = Transformer::new(file_path, true)?;
    transformer.parse()
}
