use de::deserialize;
use ser::serialize;
use ty::Ty;

mod binary;
mod de;
mod ser;
mod ty;
mod varint;

type JsonValue = serde_json::Value;

fn main() {
    let my_struct = struct_def!({
        "name": Ty::String,
        "age": Ty::U64,
        "hobbies": array_def!(Ty::String),
    });

    let value = serde_json::json!({
        "name": "Alexander",
        "age": 27,
        "hobbies": [
            "music",
            "programming"
        ]
    });

    let bytes = deserialize(&my_struct, &value).unwrap();
    println!("{:?}", bytes.as_bytes());

    let new_value = serialize(&my_struct, &bytes);
    println!("{:?}", new_value);
}
