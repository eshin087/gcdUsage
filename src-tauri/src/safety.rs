//! Small, bounded reads for externally supplied data.
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub(crate) fn read_limited(reader: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Input exceeds size limit",
        ));
    }
    Ok(bytes)
}
pub(crate) fn read_file_limited(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected a regular file",
        ));
    }
    read_limited(file, limit)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn security_stops_reading_at_the_limit_without_allocating_the_full_input() {
        let mut input = io::Cursor::new(vec![0; 4096]);
        assert!(read_limited(&mut input, 64).is_err());
        assert_eq!(input.position(), 65);
        assert_eq!(read_limited(&b"small"[..], 5).unwrap(), b"small");
    }
}
