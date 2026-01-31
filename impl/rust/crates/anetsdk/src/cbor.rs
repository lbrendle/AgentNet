use crate::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum CborValue {
    Unsigned(u64),
    Negative(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<CborValue>),
    Map(Vec<(CborValue, CborValue)>),
    Bool(bool),
    Null,
}

pub fn decode_canonical(data: &[u8]) -> Result<CborValue, Error> {
    let mut pos = 0usize;
    let value = decode_value(data, &mut pos)?;
    if pos != data.len() {
        return Err(Error::TrailingBytes);
    }
    Ok(value)
}

pub fn encode_canonical(value: &CborValue) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    encode_value(value, &mut out)?;
    Ok(out)
}

fn decode_value(data: &[u8], pos: &mut usize) -> Result<CborValue, Error> {
    if *pos >= data.len() {
        return Err(Error::Cbor("unexpected end of input"));
    }
    let initial = data[*pos];
    *pos += 1;
    let major = initial >> 5;
    let addl = initial & 0x1f;
    match major {
        0 => {
            let n = read_len(data, pos, addl)?;
            Ok(CborValue::Unsigned(n))
        }
        1 => {
            let n = read_len(data, pos, addl)?;
            if n > i64::MAX as u64 {
                return Err(Error::IntegerOverflow);
            }
            let val = -1i64 - (n as i64);
            Ok(CborValue::Negative(val))
        }
        2 => {
            let len = read_len(data, pos, addl)? as usize;
            if *pos + len > data.len() {
                return Err(Error::Cbor("unexpected end of input"));
            }
            let bytes = data[*pos..*pos + len].to_vec();
            *pos += len;
            Ok(CborValue::Bytes(bytes))
        }
        3 => {
            let len = read_len(data, pos, addl)? as usize;
            if *pos + len > data.len() {
                return Err(Error::Cbor("unexpected end of input"));
            }
            let bytes = &data[*pos..*pos + len];
            let text = std::str::from_utf8(bytes).map_err(|_| Error::Utf8)?;
            *pos += len;
            Ok(CborValue::Text(text.to_string()))
        }
        4 => {
            let len = read_len(data, pos, addl)? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                let v = decode_value(data, pos)?;
                items.push(v);
            }
            Ok(CborValue::Array(items))
        }
        5 => {
            let len = read_len(data, pos, addl)? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                let k = decode_value(data, pos)?;
                let v = decode_value(data, pos)?;
                items.push((k, v));
            }
            Ok(CborValue::Map(items))
        }
        6 => Err(Error::Unsupported),
        7 => match addl {
            20 => Ok(CborValue::Bool(false)),
            21 => Ok(CborValue::Bool(true)),
            22 => Ok(CborValue::Null),
            23 => Err(Error::Unsupported),
            _ => Err(Error::Unsupported),
        },
        _ => Err(Error::Unsupported),
    }
}

fn read_len(data: &[u8], pos: &mut usize, addl: u8) -> Result<u64, Error> {
    match addl {
        v @ 0..=23 => Ok(v as u64),
        24 => {
            let v = read_u8(data, pos)? as u64;
            if v < 24 {
                return Err(Error::NonCanonicalLength);
            }
            Ok(v)
        }
        25 => {
            let v = read_u16(data, pos)? as u64;
            if v < 256 {
                return Err(Error::NonCanonicalLength);
            }
            Ok(v)
        }
        26 => {
            let v = read_u32(data, pos)? as u64;
            if v < 65536 {
                return Err(Error::NonCanonicalLength);
            }
            Ok(v)
        }
        27 => {
            let v = read_u64(data, pos)?;
            if v < 4294967296 {
                return Err(Error::NonCanonicalLength);
            }
            Ok(v)
        }
        31 => Err(Error::IndefiniteLength),
        _ => Err(Error::Unsupported),
    }
}

fn read_u8(data: &[u8], pos: &mut usize) -> Result<u8, Error> {
    if *pos + 1 > data.len() {
        return Err(Error::Cbor("unexpected end of input"));
    }
    let v = data[*pos];
    *pos += 1;
    Ok(v)
}

fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16, Error> {
    if *pos + 2 > data.len() {
        return Err(Error::Cbor("unexpected end of input"));
    }
    let v = u16::from_be_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, Error> {
    if *pos + 4 > data.len() {
        return Err(Error::Cbor("unexpected end of input"));
    }
    let v = u32::from_be_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

fn read_u64(data: &[u8], pos: &mut usize) -> Result<u64, Error> {
    if *pos + 8 > data.len() {
        return Err(Error::Cbor("unexpected end of input"));
    }
    let v = u64::from_be_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
        data[*pos + 4],
        data[*pos + 5],
        data[*pos + 6],
        data[*pos + 7],
    ]);
    *pos += 8;
    Ok(v)
}

fn encode_value(value: &CborValue, out: &mut Vec<u8>) -> Result<(), Error> {
    match value {
        CborValue::Unsigned(n) => encode_major(out, 0, *n),
        CborValue::Negative(n) => {
            if *n >= 0 {
                return Err(Error::IntegerOverflow);
            }
            let val = (-(i128::from(*n)) - 1) as i128;
            if val < 0 || val > u64::MAX as i128 {
                return Err(Error::IntegerOverflow);
            }
            encode_major(out, 1, val as u64)
        }
        CborValue::Bytes(bytes) => {
            encode_major(out, 2, bytes.len() as u64);
            out.extend_from_slice(bytes);
        }
        CborValue::Text(s) => {
            let bytes = s.as_bytes();
            encode_major(out, 3, bytes.len() as u64);
            out.extend_from_slice(bytes);
        }
        CborValue::Array(items) => {
            encode_major(out, 4, items.len() as u64);
            for v in items {
                encode_value(v, out)?;
            }
        }
        CborValue::Map(entries) => {
            let mut prepared = Vec::with_capacity(entries.len());
            for (k, v) in entries {
                let key_bytes = encode_canonical(k)?;
                prepared.push((key_bytes, k, v));
            }
            prepared.sort_by(|a, b| {
                let len_cmp = a.0.len().cmp(&b.0.len());
                if len_cmp != std::cmp::Ordering::Equal {
                    return len_cmp;
                }
                a.0.cmp(&b.0)
            });
            for i in 1..prepared.len() {
                if prepared[i - 1].0 == prepared[i].0 {
                    return Err(Error::DuplicateKey);
                }
            }
            encode_major(out, 5, prepared.len() as u64);
            for (key_bytes, _k, v) in prepared {
                out.extend_from_slice(&key_bytes);
                encode_value(v, out)?;
            }
        }
        CborValue::Bool(b) => {
            out.push(if *b { 0xf5 } else { 0xf4 });
        }
        CborValue::Null => {
            out.push(0xf6);
        }
    }
    Ok(())
}

fn encode_major(out: &mut Vec<u8>, major: u8, value: u64) {
    match value {
        0..=23 => out.push((major << 5) | (value as u8)),
        24..=0xff => {
            out.push((major << 5) | 24);
            out.push(value as u8);
        }
        0x100..=0xffff => {
            out.push((major << 5) | 25);
            out.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push((major << 5) | 26);
            out.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            out.push((major << 5) | 27);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

// Tests are executed via vector-based conformance in the anet-vectors crate.
