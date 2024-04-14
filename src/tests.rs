#![cfg(test)]

use crate::array_def;
use crate::flatbin::{Builder, Flatbin, FlatbinBuf};
use crate::slow::{deserialize, deserialize_alloc, serialize};
use crate::struct_def;
use crate::ty::Ty;
use crate::JsonValue;

#[test]
fn unexpected_type() {
    use crate::slow::Error;

    let result = deserialize_alloc(&Ty::Bool, &JsonValue::String("Hello".into()));
    assert!(matches!(result, Err(Error::UnexpectedType { .. })));

    let result = deserialize_alloc(&Ty::String, &JsonValue::Bool(true));
    assert!(matches!(result, Err(Error::UnexpectedType { .. })));
}

#[test]
fn simple_roundtrip() {
    let mut buffer = FlatbinBuf::new();

    let ty = struct_def!({
        "name": Ty::String,
        "age": Ty::U64,
        "hobbies": array_def!(Ty::String),
        "rustacean": Ty::Bool
    });

    let value = serde_json::json!({
        "name": "Alexander",
        "age": 27,
        "hobbies": [
            "music",
            "programming"
        ],
        "rustacean": true
    });

    deserialize(&ty, &value, Builder::new(&mut buffer)).unwrap();
    let new_value = serialize(&ty, &buffer).unwrap();
    assert_eq!(value, new_value);
}

#[test]
fn garbage_data() {
    let ty = struct_def!({
        "name": Ty::String,
        "age": Ty::U64,
        "hobbies": array_def!(Ty::String),
        "rustacean": Ty::Bool
    });

    let result = serialize(&ty, Flatbin::from_bytes(&[5, 1, 99, 254, 0, 0, 11]));
    assert!(result.is_err());
}
