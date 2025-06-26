use bytes::Bytes;
use std::io::{Error, Write};

pub struct FileWal {
    wal_file: std::fs::File,
    metadata_file: std::fs::File,
    uncommitted: Vec<WalEntry>,
    offset: usize,
    size: usize,
}

struct WalEntry {
    payload: Bytes,
}

impl WalEntry {
    fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len() as u32;
        let checksum = checksum_xor_u32(&self.payload);
        
        let mut encoded = Vec::with_capacity(8 + self.payload.len());
        encoded.extend_from_slice(&payload_len.to_ne_bytes());
        encoded.extend_from_slice(&checksum.to_ne_bytes());
        encoded.extend_from_slice(&self.payload);
        
        encoded
    }
    
    fn len(&self) -> usize {
        8 + self.payload.len() // 4 bytes for payload length, 4 bytes for checksum
    }
}

pub trait Wal {
    fn write(&mut self, record: &[u8]) -> Result<(), Error>;

    fn read(&self, offset: usize) -> Result<Vec<u8>, Error>;

    fn size(&self) -> usize;
    
    fn clean_until(&mut self, offset: usize) -> Result<(), Error>;
}

impl Wal for FileWal {
    fn write(&mut self, record: &[u8]) -> Result<(), Error> {
        let entry = WalEntry {
            payload: Bytes::from(record.to_vec()),
        };
        
        let entry_len = entry.len();
        self.uncommitted.push(entry);
        
        if self.uncommitted.len() > 3 {
            self.flush()?;
        }
        
        self.offset += entry_len;
        self.size += 1;
        Ok(())
    }

    fn read(&self, offset: usize) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn size(&self) -> usize {
        self.size
    }

    fn clean_until(&mut self, offset: usize) -> Result<(), Error> {
        todo!()
    }
}

impl FileWal {
    pub fn new(wal_path: &str, metadata_path: &str) -> Result<Self, Error> {
        let wal_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(wal_path)?;
        let metadata_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(metadata_path)?;

        Ok(FileWal {
            wal_file,
            metadata_file,
            uncommitted: Vec::new(),
            offset: 0,
            size: 0,
        })
    }

    fn flush(&mut self) -> Result<(), Error> {
        for entry in &self.uncommitted {
            self.wal_file.write_all(&entry.encode())?;
        }
        self.wal_file.flush()?;
        self.uncommitted.clear();
        self.metadata_file.set_len(0)?;
        self.metadata_file.write(&self.offset.to_ne_bytes())?;
        Ok(())
    }
}

fn checksum_xor_u32(bytes: &[u8]) -> usize {
    bytes.iter().fold(0u8, |acc, &b| acc ^ b) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempfile;

    #[test]
    fn test_simple_write() {
        let mut wal = FileWal {
            wal_file: tempfile().unwrap(),
            metadata_file: tempfile().unwrap(),
            uncommitted: Vec::new(),
            offset: 0,
            size: 0,
        };

        let data = "some data goes here 100";
        wal.write(data.as_bytes()).unwrap();
        assert_eq!(wal.size(), 1);
        assert_eq!(wal.offset, data.len());
        assert_eq!(wal.read(0).unwrap(), b"some data goes here 100");
    }
    
    
    #[test]
    fn test_read_at_offset() {
        let data1 = "first entry";
        let data2 = "second entry";
        let mut wal = FileWal {
            wal_file: tempfile().unwrap(),
            metadata_file: tempfile().unwrap(),
            uncommitted: Vec::new(),
            offset: 0,
            size: 0,
        };
        
        wal.write(data1.as_bytes()).unwrap();
        wal.write(data2.as_bytes()).unwrap();

        assert_eq!(wal.read(0).unwrap(), b"first entry");
        assert_eq!(wal.read(data1.len()).unwrap(), b"second entry");
    }
}