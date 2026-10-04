//! Small deterministic binary codec used by consensus data structures.
//!
//! This is deliberately minimal. It is not a general serialization framework.

use crate::ProtocolError;

pub trait Encode {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtocolError>;
}

pub struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        if self.remaining() < 1 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let value = self.bytes[self.offset];
        self.offset += 1;
        Ok(value)
    }

    pub fn read_u16_le(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.read_array()?))
    }

    pub fn read_u32_le(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    pub fn read_u64_le(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }

    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ProtocolError> {
        if self.remaining() < N {
            return Err(ProtocolError::UnexpectedEof);
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.bytes[self.offset..self.offset + N]);
        self.offset += N;
        Ok(out)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ProtocolError> {
        if self.remaining() < len {
            return Err(ProtocolError::UnexpectedEof);
        }
        let out = &self.bytes[self.offset..self.offset + len];
        self.offset += len;
        Ok(out)
    }

    pub fn finish(self) -> Result<(), ProtocolError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(ProtocolError::TrailingBytes)
        }
    }
}

pub fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

pub fn put_u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn put_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn put_u64_le(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn put_len_u32(out: &mut Vec<u8>, len: usize) -> Result<(), ProtocolError> {
    let len = u32::try_from(len).map_err(|_| ProtocolError::LengthOverflow)?;
    put_u32_le(out, len);
    Ok(())
}

pub fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ProtocolError> {
    put_len_u32(out, bytes.len())?;
    out.extend_from_slice(bytes);
    Ok(())
}

pub fn read_bytes_u32<'a>(reader: &mut Reader<'a>) -> Result<&'a [u8], ProtocolError> {
    let len = reader.read_u32_le()? as usize;
    reader.read_bytes(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_rejects_trailing_bytes() {
        let mut reader = Reader::new(&[1, 2]);
        assert_eq!(reader.read_u8().unwrap(), 1);
        assert!(matches!(reader.finish(), Err(ProtocolError::TrailingBytes)));
    }

    #[test]
    fn bytes_round_trip() {
        let mut encoded = Vec::new();
        put_bytes(&mut encoded, b"memobi").unwrap();
        let mut reader = Reader::new(&encoded);
        assert_eq!(read_bytes_u32(&mut reader).unwrap(), b"memobi");
        reader.finish().unwrap();
    }
}
