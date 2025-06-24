use bytes::{Buf, BufMut, Bytes, BytesMut};

pub struct FileWal {
    file: std::fs::File,
    uncommitted: Vec<WalEntry>,
    offset: usize,
}

struct MemoryWal {
    data: BytesMut,
    offset: usize,
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

impl Wal for MemoryWal {
    fn write(&mut self, record: &[u8]) -> Result<(), std::io::Error> {

        let entry = WalEntry {
            offset: self.offset,
            data: Bytes::from(record.to_vec()),
        };

        self.data.extend_from_slice(&record);
        self.offset += record.len();
        Ok(())
    }

    fn read(&self, offset: usize) -> Result<Vec<u8>, std::io::Error> {
        if offset + 3 > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Offset out of bounds",
            ));
        }
        Ok(self.data[offset..offset + 3].to_vec())
    }

    fn size(&self) -> Result<usize, std::io::Error> {
        Ok(self.data.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        // No-op for in-memory WAL
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_wal() {
        let mut wal = MemoryWal {
            data: Vec::new(),
            offset: 0,
        };

        wal.write(entry).unwrap();
        assert_eq!(wal.size().unwrap(), 3);
        assert_eq!(wal.read(0).unwrap(), vec![1, 2, 3]);
    }
}