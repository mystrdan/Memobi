//! Persistent canonical-header storage foundation.
//!
//! This layer deliberately stores consensus headers separately from volatile
//! UTXO state. It provides crash-tolerant append/reopen semantics needed before
//! full block/state persistence is introduced.

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    BlockHeight, ProtocolError,
    block::BlockHeader,
    codec::{Reader, put_u32_le},
    hash::Hash32,
};

const MAGIC: &[u8; 8] = b"MEMOHDR\0";
const VERSION: u32 = 1;
const HEADER_RECORD_BYTES: usize = 104;
const RECORD_PREFIX_BYTES: usize = 4 + 4 + 32;

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    Protocol(ProtocolError),
    Corrupt(&'static str),
}

impl From<io::Error> for StorageError {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}
impl From<ProtocolError> for StorageError {
    fn from(value: ProtocolError) -> Self { Self::Protocol(value) }
}

pub struct HeaderStore {
    path: PathBuf,
    file: File,
    count: u64,
}

impl HeaderStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new().create(true).read(true).append(true).open(&path)?;
        let len = file.metadata()?.len();
        let count = if len == 0 {
            file.write_all(MAGIC)?;
            file.write_all(&VERSION.to_le_bytes())?;
            file.flush()?;
            0
        } else {
            if len < 12 { return Err(StorageError::Corrupt("truncated store header")); }
            let mut prefix = [0u8; 12];
            file.read_exact_at(&mut prefix, 0)?;
            if &prefix[..8] != MAGIC { return Err(StorageError::Corrupt("invalid store magic")); }
            if u32::from_le_bytes(prefix[8..12].try_into().unwrap()) != VERSION {
                return Err(StorageError::Corrupt("unsupported store version"));
            }
            let payload = len - 12;
            if payload % (RECORD_PREFIX_BYTES as u64 + HEADER_RECORD_BYTES as u64) != 0 {
                return Err(StorageError::Corrupt("truncated header record"));
            }
            payload / (RECORD_PREFIX_BYTES as u64 + HEADER_RECORD_BYTES as u64)
        };
        Ok(Self { path, file, count })
    }

    pub fn path(&self) -> &Path { &self.path }
    pub fn len(&self) -> u64 { self.count }

    pub fn append(&mut self, header: &BlockHeader) -> Result<(), StorageError> {
        let bytes = header.encode_to_vec()?;
        if bytes.len() != HEADER_RECORD_BYTES {
            return Err(StorageError::Corrupt("unexpected header encoding length"));
        }
        let id = header.block_id()?;
        let mut record = Vec::with_capacity(RECORD_PREFIX_BYTES + bytes.len());
        put_u32_le(&mut record, VERSION);
        put_u32_le(&mut record, bytes.len() as u32);
        record.extend_from_slice(id.as_bytes());
        record.extend_from_slice(&bytes);
        self.file.write_all(&record)?;
        self.file.flush()?;
        self.file.sync_data()?;
        self.count += 1;
        Ok(())
    }

    pub fn read_all(&mut self) -> Result<Vec<BlockHeader>, StorageError> {
        let mut file = File::open(&self.path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        if bytes.len() < 12 || &bytes[..8] != MAGIC {
            return Err(StorageError::Corrupt("invalid store"));
        }
        let mut reader = Reader::new(&bytes[12..]);
        let mut headers = Vec::with_capacity(self.count as usize);
        while reader.remaining() > 0 {
            if reader.remaining() < RECORD_PREFIX_BYTES + HEADER_RECORD_BYTES {
                return Err(StorageError::Corrupt("truncated record"));
            }
            let version = reader.read_u32_le()?;
            let len = reader.read_u32_le()? as usize;
            if version != VERSION || len != HEADER_RECORD_BYTES {
                return Err(StorageError::Corrupt("invalid record"));
            }
            let expected_id = Hash32(reader.read_array()?);
            let encoded = reader.read_bytes(len)?;
            let mut h = Reader::new(encoded);
            let header = BlockHeader {
                version: h.read_u32_le()?,
                previous_block: Hash32(h.read_array()?),
                height: BlockHeight(h.read_u64_le()?),
                timestamp: h.read_u64_le()?,
                target: h.read_u64_le()?,
                poarm_version: h.read_u32_le()?,
                poarm_nonce: h.read_u64_le()?,
                transaction_root: Hash32(h.read_array()?),
            };
            h.finish()?;
            if header.block_id()? != expected_id {
                return Err(StorageError::Corrupt("header checksum mismatch"));
            }
            headers.push(header);
        }
        Ok(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path() -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("memobi-headers-{n}.db"))
    }

    fn sample(height: u64) -> BlockHeader {
        BlockHeader {
            version: 1,
            previous_block: Hash32([height.saturating_sub(1) as u8; 32]),
            height: BlockHeight(height),
            timestamp: 1_700_000_000 + height * 10,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: height,
            transaction_root: Hash32([height as u8; 32]),
        }
    }

    #[test]
    fn append_reopen_and_read_headers() {
        let path = temp_path();
        {
            let mut store = HeaderStore::open(&path).unwrap();
            store.append(&sample(0)).unwrap();
            store.append(&sample(1)).unwrap();
        }
        {
            let mut store = HeaderStore::open(&path).unwrap();
            assert_eq!(store.len(), 2);
            assert_eq!(store.read_all().unwrap(), vec![sample(0), sample(1)]);
        }
        let _ = std::fs::remove_file(path);
    }
}
