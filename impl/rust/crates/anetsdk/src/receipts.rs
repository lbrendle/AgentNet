use crate::{decode_canonical, parse_receipt_payload, sha256, verify_ed25519_hash, Error};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub struct ReceiptRecord {
    pub payload: Vec<u8>,
    pub receipt_hash: [u8; 32],
    pub signature: Vec<u8>,
}

pub struct ReceiptLog {
    file: File,
    last_hash: [u8; 32],
    last_seq: u64,
}

impl ReceiptLog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| Error::Io(e.to_string()))?;

        let mut log = ReceiptLog {
            file,
            last_hash: [0u8; 32],
            last_seq: 0,
        };

        log.replay()?;
        Ok(log)
    }

    pub fn append(&mut self, payload: &[u8], signature: &[u8]) -> Result<ReceiptRecord, Error> {
        self.append_internal(payload, signature, None)
    }

    pub fn append_verified(
        &mut self,
        payload: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<ReceiptRecord, Error> {
        self.append_internal(payload, signature, Some(public_key))
    }

    fn append_internal(
        &mut self,
        payload: &[u8],
        signature: &[u8],
        public_key: Option<&[u8]>,
    ) -> Result<ReceiptRecord, Error> {
        let value = decode_canonical(payload)?;
        let receipt = parse_receipt_payload(&value)?;

        if receipt.seq != self.last_seq.saturating_add(1) {
            return Err(Error::Cbor("receipt sequence mismatch"));
        }
        if receipt.prev_hash != self.last_hash {
            return Err(Error::Cbor("receipt prev_hash mismatch"));
        }

        let receipt_hash = sha256(payload);

        if let Some(pk) = public_key {
            verify_ed25519_hash(pk, &receipt_hash, signature)?;
        }

        self.write_record(payload, signature)?;
        self.last_hash = receipt_hash;
        self.last_seq = receipt.seq;

        Ok(ReceiptRecord {
            payload: payload.to_vec(),
            receipt_hash,
            signature: signature.to_vec(),
        })
    }

    pub fn last_hash(&self) -> [u8; 32] {
        self.last_hash
    }

    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    fn replay(&mut self) -> Result<(), Error> {
        self.file.seek(SeekFrom::Start(0)).map_err(|e| Error::Io(e.to_string()))?;
        loop {
            let mut len_buf = [0u8; 4];
            if self.file.read(&mut len_buf).map_err(|e| Error::Io(e.to_string()))? == 0 {
                break;
            }
            let payload_len = u32::from_be_bytes(len_buf) as usize;
            let mut payload = vec![0u8; payload_len];
            self.file.read_exact(&mut payload).map_err(|e| Error::Io(e.to_string()))?;

            let mut sig_len_buf = [0u8; 4];
            self.file.read_exact(&mut sig_len_buf).map_err(|e| Error::Io(e.to_string()))?;
            let sig_len = u32::from_be_bytes(sig_len_buf) as usize;
            let mut signature = vec![0u8; sig_len];
            self.file.read_exact(&mut signature).map_err(|e| Error::Io(e.to_string()))?;

            let value = decode_canonical(&payload)?;
            let receipt = parse_receipt_payload(&value)?;

            if receipt.seq != self.last_seq.saturating_add(1) {
                return Err(Error::Cbor("receipt sequence mismatch"));
            }
            if receipt.prev_hash != self.last_hash {
                return Err(Error::Cbor("receipt prev_hash mismatch"));
            }

            let receipt_hash = sha256(&payload);
            self.last_hash = receipt_hash;
            self.last_seq = receipt.seq;
        }

        self.file.seek(SeekFrom::End(0)).map_err(|e| Error::Io(e.to_string()))?;
        Ok(())
    }

    fn write_record(&mut self, payload: &[u8], signature: &[u8]) -> Result<(), Error> {
        self.file
            .write_all(&(payload.len() as u32).to_be_bytes())
            .map_err(|e| Error::Io(e.to_string()))?;
        self.file.write_all(payload).map_err(|e| Error::Io(e.to_string()))?;
        self.file
            .write_all(&(signature.len() as u32).to_be_bytes())
            .map_err(|e| Error::Io(e.to_string()))?;
        self.file.write_all(signature).map_err(|e| Error::Io(e.to_string()))?;
        self.file.sync_all().map_err(|e| Error::Io(e.to_string()))?;
        Ok(())
    }
}
