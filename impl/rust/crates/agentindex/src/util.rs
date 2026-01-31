use anetsdk::CborValue;
use serde_json::{json, Value};

pub fn cbor_to_json_value(value: &CborValue) -> Value {
    match value {
        CborValue::Unsigned(n) => json!(n),
        CborValue::Negative(n) => json!(n),
        CborValue::Bytes(bytes) => json!({
            "_type": "bytes",
            "hex": hex::encode(bytes),
        }),
        CborValue::Text(text) => json!(text),
        CborValue::Array(items) => {
            let values = items.iter().map(cbor_to_json_value).collect();
            Value::Array(values)
        }
        CborValue::Map(entries) => {
            let pairs = entries
                .iter()
                .map(|(k, v)| json!([cbor_to_json_value(k), cbor_to_json_value(v)]))
                .collect::<Vec<_>>();
            json!({
                "_type": "map",
                "entries": pairs,
            })
        }
        CborValue::Bool(b) => json!(b),
        CborValue::Null => Value::Null,
    }
}
