pub use de::*;
pub use ser::*;

mod de;
mod ser;

#[cfg(test)]
mod test {
    use super::{deserialize, serialize};
    use crate::{array_def, struct_def, ty::Ty, JsonValue};

    #[test]
    fn bool_roundtrip() {
        let ty = Ty::Bool;
        let value = JsonValue::Bool(false);

        let bytes = deserialize(&ty, &value.to_string()).unwrap();
        let new_value = serialize(serde_json::value::Serializer, &ty, &bytes).unwrap();
        assert_eq!(value, new_value);
    }

    #[test]
    fn simple_roundtrip() {
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

        let bytes = deserialize(&ty, &value.to_string()).unwrap();
        println!("{:?}", bytes);
        let new_value = serialize(serde_json::value::Serializer, &ty, &bytes).unwrap();
        assert_eq!(value, new_value);
    }
}
