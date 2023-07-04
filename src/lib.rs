#![warn(missing_docs)]
//! Processing yaml with include documents through `!include <path>` tag.
//!
//! ## Features
//!
//! - include and parse recursively `yaml` (and `json`) files
//! - include `markdown` and `txt` text files
//! - include other types as `base64` encoded binary data.
//! - optionaly handle gracefully circular references with `!circular` tag
//!
//! ## Example
//! ```
//! use std::path::PathBuf;
//! use yaml_include::Transformer;
//!
//! let path = PathBuf::from("data/sample/main.yml");
//! if let Ok(transformer) = Transformer::new(path, false) {
//!     println!("{}", transformer);
//! };
//! ```
mod helpers;
mod transformer;

pub use transformer::*;
