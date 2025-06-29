use std::io::{Error, Read, Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct FileWal {
    wal_file: std::fs::File,
    metadata_file: std::fs::File,
    uncommitted: Vec<WalEntry>,
    offset: usize,
    size: usize,
}

#[derive(Debug)]
struct WalEntry {
    payload: Vec<u8>,
}

impl WalEntry {
    fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len();
        let checksum = checksum_xor_u32(&self.payload);

        let size_of_payload_len = size_of_val(&payload_len);
        let size_of_checksum = size_of_val(&checksum);

        let mut encoded =
            Vec::with_capacity(size_of_payload_len + size_of_checksum + self.payload.len());
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
            payload: record.to_vec(),
        };

        let entry_len = entry.len();
        self.uncommitted.push(entry);

        if self.uncommitted.len() > 3 {
            self.flush()?;
        }

        self.offset += 8 + entry_len;
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

    fn as_vec(&mut self) -> Result<Vec<WalEntry>, Error> {
        let mut vec = Vec::with_capacity(self.size);

        self.wal_file.seek(SeekFrom::Start(0))?;

        let mut whole_file = Vec::new();
        self.wal_file.read_to_end(&mut whole_file)?;
        self.wal_file.seek(SeekFrom::Start(0))?;

        for _ in 0..self.size {
            let mut len_buf = [0u8; 8];

            self.wal_file.read_exact(&mut len_buf)?;
            let payload_len = usize::from_ne_bytes(len_buf);

            let mut checksum_buf = [0u8; 8];
            self.wal_file.read_exact(&mut checksum_buf)?;
            let checksum = usize::from_ne_bytes(checksum_buf);

            let mut payload_buf = vec![0u8; payload_len];
            self.wal_file.read_exact(&mut payload_buf)?;

            let actual_checksum = checksum_xor_u32(&payload_buf);

            if actual_checksum != checksum as usize {
                return Err(Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Checksum mismatch",
                ));
            }

            vec.push(WalEntry {
                payload: payload_buf,
            });
        }

        let result = Ok(vec);
        result
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

    #[test]
    fn test_as_vec() {
        let mut wal = FileWal {
            wal_file: tempfile().unwrap(),
            metadata_file: tempfile().unwrap(),
            uncommitted: Vec::new(),
            offset: 0,
            size: 0,
        };

        let data1 = "first entry";
        let data2 = "second entry";

        wal.write(data1.as_bytes()).unwrap();
        wal.write(data2.as_bytes()).unwrap();
        wal.flush().unwrap();

        let entries = wal.as_vec().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].payload, Bytes::from(data1));
        assert_eq!(entries[1].payload, Bytes::from(data2));
    }
}
