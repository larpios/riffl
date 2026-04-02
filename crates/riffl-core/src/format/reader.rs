//! Shared binary reader utility for format parsers.
//!
//! Eliminates duplicated `read_u8` / `read_u16_le` / etc. free functions that
//! previously existed identically in each of the four format parsers (IT, XM,
//! S3M, ProTracker). All methods return `Result<T, FormatError::TruncatedFile>`
//! so malformed or truncated files produce a clean error instead of an index
//! panic.

use super::error::{FormatError, FormatResult};

/// Stateful cursor over a byte slice for sequential binary parsing.
pub struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Create a reader starting at a specific offset.
    pub fn at(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    /// Current read position.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Set the read position directly.
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Number of bytes remaining from the current position.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Peek at the next `n` bytes without advancing the position.
    pub fn peek_bytes(&self, n: usize) -> Option<&[u8]> {
        let end = self.pos + n;
        if end <= self.data.len() {
            Some(&self.data[self.pos..end])
        } else {
            None
        }
    }

    /// Advance the position by `n` bytes without reading.
    pub fn skip(&mut self, n: usize) -> FormatResult<()> {
        let new_pos = self.pos + n;
        if new_pos > self.data.len() {
            return Err(FormatError::TruncatedFile(format!(
                "skip({n}) at offset {}: only {} bytes remain",
                self.pos,
                self.remaining()
            )));
        }
        self.pos = new_pos;
        Ok(())
    }

    pub fn read_u8(&mut self) -> FormatResult<u8> {
        if self.pos >= self.data.len() {
            return Err(FormatError::TruncatedFile(format!(
                "read_u8 at offset {}: file ended",
                self.pos
            )));
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub fn read_i8(&mut self) -> FormatResult<i8> {
        self.read_u8().map(|v| v as i8)
    }

    pub fn read_u16_le(&mut self) -> FormatResult<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(FormatError::TruncatedFile(format!(
                "read_u16_le at offset {}: need 2 bytes, {} remain",
                self.pos,
                self.remaining()
            )));
        }
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    pub fn read_u16_be(&mut self) -> FormatResult<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(FormatError::TruncatedFile(format!(
                "read_u16_be at offset {}: need 2 bytes, {} remain",
                self.pos,
                self.remaining()
            )));
        }
        let v = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    pub fn read_i16_le(&mut self) -> FormatResult<i16> {
        self.read_u16_le().map(|v| v as i16)
    }

    pub fn read_u32_le(&mut self) -> FormatResult<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(FormatError::TruncatedFile(format!(
                "read_u32_le at offset {}: need 4 bytes, {} remain",
                self.pos,
                self.remaining()
            )));
        }
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    /// Read `len` bytes as a UTF-8 string, trimming null bytes and whitespace.
    pub fn read_string(&mut self, len: usize) -> FormatResult<String> {
        let end = self.pos + len;
        if end > self.data.len() {
            return Err(FormatError::TruncatedFile(format!(
                "read_string({len}) at offset {}: need {len} bytes, {} remain",
                self.pos,
                self.remaining()
            )));
        }
        let s = String::from_utf8_lossy(&self.data[self.pos..end])
            .trim_end_matches('\0')
            .trim()
            .to_string();
        self.pos += len;
        Ok(s)
    }

    /// Read exactly `n` bytes as a slice reference without copying.
    pub fn read_bytes(&mut self, n: usize) -> FormatResult<&'a [u8]> {
        let end = self.pos + n;
        if end > self.data.len() {
            return Err(FormatError::TruncatedFile(format!(
                "read_bytes({n}) at offset {}: need {n} bytes, {} remain",
                self.pos,
                self.remaining()
            )));
        }
        let slice = &self.data[self.pos..end];
        self.pos += n;
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_primitives() {
        let data = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06];
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_u8().unwrap(), 0x01);
        assert_eq!(r.read_u16_le().unwrap(), 0x0302);
        assert_eq!(r.read_u16_be().unwrap(), 0x0405);
        assert_eq!(r.remaining(), 1);
    }

    #[test]
    fn truncated_returns_error() {
        let data = [0x01u8];
        let mut r = BinaryReader::new(&data);
        assert!(r.read_u16_le().is_err());
    }

    #[test]
    fn read_string_trims_nulls() {
        let data = b"hello\0\0\0";
        let mut r = BinaryReader::new(data);
        assert_eq!(r.read_string(8).unwrap(), "hello");
    }
}
