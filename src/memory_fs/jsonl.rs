use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::error::Result;

pub fn append_jsonl_line(path: &Path, line: &str) -> Result<usize> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let line_no = existing_line_count(path)? + 1;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    if file.metadata()?.len() > 0 {
        if !ends_with_newline(path)? {
            file.write_all(b"\n")?;
        }
    }

    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(line_no)
}

fn existing_line_count(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().count())
}

fn ends_with_newline(path: &Path) -> Result<bool> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Ok(true);
    }

    let mut file = File::open(path)?;
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::End(-1))?;
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte)?;
    Ok(byte[0] == b'\n')
}
