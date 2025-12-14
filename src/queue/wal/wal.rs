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
        let checksum = checksum_xor(&self.payload);

        let mut encoded = Vec::with_capacity(std::mem::size_of::<usize>() * 2 + self.payload.len());
        encoded.extend_from_slice(&payload_len.to_ne_bytes());
        encoded.extend_from_slice(&checksum.to_ne_bytes());
        encoded.extend_from_slice(&self.payload);

        encoded
    }

    fn len(&self) -> usize {
        std::mem::size_of::<usize>() * 2 + self.payload.len() // usize bytes for payload length, usize bytes for checksum
    }
}

pub trait Wal {
    fn write(&mut self, record: &[u8]) -> Result<(), Error>;

    fn read(&self, index: usize) -> Result<Vec<u8>, Error>;

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

        // TODO: This really doesn't do anything, we're just putting it in a queue to clear it...
        if self.uncommitted.len() > 0 {
            self.flush()?;
        }

        self.offset += entry_len;
        println!("current offset: {}", self.offset);
        self.size += 1;
        Ok(())
    }

    fn read(&self, index: usize) -> Result<Vec<u8>, Error> {
        if index >= self.size {
            return Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Index out of range, WAL is length {}", self.size),
            ));
        }

        let mut file = self.wal_file.try_clone()?;
        file.seek(SeekFrom::Start(0))?;

        // Skip to the desired entry
        for i in 0..=index {
            // Read payload length
            let mut len_buf = [0u8; std::mem::size_of::<usize>()];
            file.read_exact(&mut len_buf)?;
            let payload_len = usize::from_ne_bytes(len_buf);

            // Read checksum
            let mut checksum_buf = [0u8; std::mem::size_of::<usize>()];
            file.read_exact(&mut checksum_buf)?;
            let stored_checksum = usize::from_ne_bytes(checksum_buf);

            // If this is the entry we want, read and return it
            if i == index {
                let mut payload = vec![0u8; payload_len];
                file.read_exact(&mut payload)?;

                // Verify checksum
                let actual_checksum = checksum_xor(&payload);
                if actual_checksum != stored_checksum {
                    return Err(Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Checksum mismatch",
                    ));
                }

                return Ok(payload);
            }

            // Skip this entry's payload
            file.seek(SeekFrom::Current(payload_len as i64))?;
        }

        unreachable!()
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
            let mut len_buf = [0u8; std::mem::size_of::<usize>()];

            self.wal_file.read_exact(&mut len_buf)?;
            let payload_len = usize::from_ne_bytes(len_buf);

            let mut checksum_buf = [0u8; std::mem::size_of::<usize>()];
            self.wal_file.read_exact(&mut checksum_buf)?;
            let checksum = usize::from_ne_bytes(checksum_buf);

            let mut payload_buf = vec![0u8; payload_len];
            self.wal_file.read_exact(&mut payload_buf)?;

            let actual_checksum = checksum_xor(&payload_buf);

            if actual_checksum != checksum {
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

fn checksum_xor(bytes: &[u8]) -> usize {
    bytes.iter().fold(0u8, |acc, &b| acc ^ b) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
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
        assert_eq!(wal.offset, data.len() + std::mem::size_of::<usize>() * 2);
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
        assert_eq!(wal.read(1).unwrap(), b"second entry");
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
