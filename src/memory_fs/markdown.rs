use std::path::Path;

use crate::error::Result;

use super::atomic;

pub fn write_markdown_file(path: &Path, contents: &str) -> Result<()> {
    atomic::write_text_atomic(path, contents)
}
