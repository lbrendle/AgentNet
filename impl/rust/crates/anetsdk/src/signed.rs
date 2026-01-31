use crate::{CborValue, Error};

pub(crate) fn split_signed_map(
    value: &CborValue,
    sig_key: u64,
) -> Result<(Vec<(CborValue, CborValue)>, Vec<u8>), Error> {
    let entries = match value {
        CborValue::Map(entries) => entries.clone(),
        _ => return Err(Error::Cbor("expected map")),
    };
    let mut payload_entries = Vec::with_capacity(entries.len());
    let mut signature: Option<Vec<u8>> = None;
    for (k, v) in entries {
        if let CborValue::Unsigned(n) = &k {
            if *n == sig_key {
                if signature.is_some() {
                    return Err(Error::Cbor("duplicate signature key"));
                }
                match v {
                    CborValue::Bytes(bytes) => signature = Some(bytes),
                    _ => return Err(Error::Cbor("signature must be bytes")),
                }
                continue;
            }
        }
        payload_entries.push((k, v));
    }
    let sig = signature.ok_or(Error::Cbor("missing signature"))?;
    if sig.len() != 64 {
        return Err(Error::Cbor("invalid signature length"));
    }
    Ok((payload_entries, sig))
}

pub(crate) fn with_signature(
    payload: &CborValue,
    sig_key: u64,
    signature: Vec<u8>,
) -> Result<CborValue, Error> {
    let mut entries = match payload {
        CborValue::Map(entries) => entries.clone(),
        _ => return Err(Error::Cbor("expected map")),
    };
    entries.push((CborValue::Unsigned(sig_key), CborValue::Bytes(signature)));
    Ok(CborValue::Map(entries))
}
