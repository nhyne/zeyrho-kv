use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::{Read, Write};

pub struct FileWal {
    wal_file: std::fs::File,
    metadata_file: std::fs::File,
    uncommitted: Vec<WalEntry>,
    offset: usize,
    size: usize,
}

struct WalEntry {
    offset: usize,
    data: Bytes,
}

pub trait Wal {
    fn write(&mut self, record: &[u8]) -> Result<(), std::io::Error>;

    fn read(&self, offset: usize) -> Result<Vec<u8>, std::io::Error>;

    fn size(&self) -> Result<usize, std::io::Error>;

    fn flush(&mut self) -> Result<(), std::io::Error>;
}

impl Wal for FileWal {
    fn write(&mut self, record: &[u8]) -> Result<(), std::io::Error> {
        let entry = WalEntry {
            offset: self.offset,
            data: Bytes::from(record.to_vec()),
        };
        self.uncommitted.push(entry);
        self.offset += record.len();
        self.size += 1;
        Ok(())
    }

    fn read(&self, offset: usize) -> Result<Vec<u8>, std::io::Error> {
        if offset >= self.offset {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Offset out of bounds",
            ));
        }
        let mut data = Vec::new();
        for entry in &self.uncommitted {
            if entry.offset == offset {
                data.extend_from_slice(&entry.data);
                break;
            }
        }
        Ok(data)
    }

    fn size(&self) -> Result<usize, std::io::Error> {
        Ok(self.size)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        for entry in &self.uncommitted {
            self.wal_file.write_all(&entry.data)?;
        }
        self.uncommitted.clear();
        self.metadata_file.set_len(0)?;
        self.metadata_file.write(&self.offset.to_ne_bytes())?;
        Ok(())
    }
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
        assert_eq!(wal.size().unwrap(), 1);
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