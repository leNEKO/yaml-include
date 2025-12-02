use std::fs;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobMatcher};
use walkdir::WalkDir;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum GlobError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[cfg(feature = "glob")]
    #[error("Invalid glob pattern: {0}")]
    GlobPatternError(#[from] globset::Error),

    #[cfg(feature = "glob")]
    #[error("Unable to read filesystem: {0}")]
    FsError(#[from] walkdir::Error),
}

pub type Result<T> = std::result::Result<T, GlobError>;

pub struct Glob {
    glob: GlobMatcher,
    pattern: String,
    context: PathBuf,
}

impl Glob {
    pub fn new(pattern: &str, context: impl AsRef<Path>) -> Result<Self> {
        Ok(Glob {
            glob: GlobBuilder::new(pattern)
                .literal_separator(true)
                .case_insensitive(false)
                .build()?
                .compile_matcher(),
            pattern: pattern.into(),
            context: PathBuf::from(context.as_ref()),
        })
    }

    /// Returns true if `path` looks like a glob pattern (e.g. "*.rs", "src/**/*.rs"),
    /// and false if it looks like a plain path (e.g. "src/main.rs").
    ///
    /// Escape rules:
    /// - `\*`, `\?`, `\[`, `\]`, `\{`, `\}` are treated as literals, not glob syntax.
    pub fn is_fixed(&self) -> bool {
        let mut escaped = false;

        for c in self.pattern.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            match c {
                '\\' => escaped = true,
                '*' | '?' | '[' | ']' | '{' | '}' => return false,
                _ => {}
            }
        }
        true
    }

    /// Create an iterator over all entries under `context` that match the glob pattern.
    pub fn iter(&self) -> GlobIterator<'_> {
        GlobIterator {
            walkdir_iter: WalkDir::new(&self.context).into_iter(),
            glob: self,
        }
    }
}

#[derive(Debug)]
pub struct GlobEntry {
    pub full_path: PathBuf,
    pub relative: Option<PathBuf>,
}

pub struct GlobIterator<'a> {
    walkdir_iter: walkdir::IntoIter,
    glob: &'a Glob,
}

impl<'a> Iterator for GlobIterator<'a> {
    type Item = Result<GlobEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        for entry_res in self.walkdir_iter.by_ref() {
            match entry_res {
                Err(err) => {
                    return Some(Err(err.into()));
                }
                Ok(entry) => {
                    let full_path = match fs::canonicalize(entry.path()) {
                        Ok(path) => path,
                        Err(err) => {
                            return Some(Err(err.into()));
                        }
                    };

                    // Try to get the relative path
                    let rel = match full_path.strip_prefix(&self.glob.context) {
                        Ok(rel) => Some(PathBuf::from(rel)),
                        Err(_) => None,
                    };

                    let globbed_matched = match &rel {
                        Some(rel) => self.glob.glob.is_match(rel),
                        None => self.glob.glob.is_match(&full_path),
                    };
                    if globbed_matched {
                        return Some(Ok(GlobEntry {
                            full_path,
                            relative: rel,
                        }));
                    }
                }
            }
        }
        None
    }
}

impl<'a> IntoIterator for &'a Glob {
    type Item = Result<GlobEntry>;
    type IntoIter = GlobIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
