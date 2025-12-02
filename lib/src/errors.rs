use std::path::PathBuf;

use serde_yaml_ng as serde_yaml;

use crate::glob;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to include file {0}: {1}")]
    IncludeError(PathBuf, String),

    #[error("Unable to find parent folder of file {0}")]
    NoParentError(PathBuf),

    #[error("Invalid string value: {0}")]
    InvalidStringValue(String),

    #[error("Unable to merge value in file include")]
    MergeError(),

    #[error("Circular reference detected for file {0}")]
    CircularReference(PathBuf),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Yaml format error: {0}")]
    ParsingError(#[from] serde_yaml::Error),

    #[cfg(feature = "glob")]
    #[error("Unable to parse glob pattern {0:?}: {1}")]
    GlobParsingError(String, String),

    #[cfg(feature = "glob")]
    #[error(transparent)]
    GlobError(#[from] glob::GlobError),
}

pub type Result<T> = std::result::Result<T, Error>;
