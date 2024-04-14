use crate::{
    flatbin::{Builder, FlatbinBuf},
    ty::Ty,
    JsonValue,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("expected {expected}, got {got}")]
    UnexpectedType { expected: &'static str, got: &'static str },
    #[error("an element in a byte array was not an integer between 0 and 255")]
    NotAByte,
    #[error("missing field: {name}")]
    MissingField { name: Box<str> },
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn deserialize(ty: &Ty, value: &JsonValue) -> Result<FlatbinBuf> {
    let mut buffer = FlatbinBuf::new();
    deserialize_into(ty, value, &mut buffer)?;
    Ok(buffer)
}

pub fn deserialize_into(ty: &Ty, value: &JsonValue, buffer: &mut FlatbinBuf) -> Result<()> {
    let builder = Builder::new(buffer);
    ty.deserialize(value, builder)?;
    Ok(())
}

impl Ty {
    pub fn deserialize(&self, value: &JsonValue, builder: Builder) -> Result<()> {
        match self {
            Ty::Bool => {
                let value = value.as_bool().ok_or(unexpected_type("a boolean", value))?;
                builder.write_bool(value);
            }
            Ty::U64 => {
                let value = value.as_u64().ok_or(unexpected_type("a non-negative integer", value))?;
                builder.write_u64(value);
            }
            Ty::I64 => {
                let value = value.as_i64().ok_or(unexpected_type("an integer", value))?;
                builder.write_i64(value);
            }
            Ty::F64 => {
                let value = value.as_f64().ok_or(unexpected_type("a number", value))?;
                builder.write_f64(value);
            }
            Ty::Bytes => {
                let value = value.as_array().ok_or(unexpected_type("a byte array", value))?;
                let bytes = value
                    .iter()
                    .map(|value| value.as_u64()?.try_into().ok())
                    .collect::<Option<Vec<u8>>>()
                    .ok_or(Error::NotAByte)?;
                builder.write_bytes(&bytes);
            }
            Ty::String => {
                let value = value.as_str().ok_or(unexpected_type("a string", value))?;
                builder.write_str(value);
            }
            Ty::Array { inner } => {
                let array = value.as_array().ok_or(unexpected_type("an array", value))?;
                let mut vector = builder.start_vector();
                for element in array {
                    inner.deserialize(element, vector.as_builder())?;
                }
                vector.end();
            }
            Ty::Struct { fields } => {
                let object = value.as_object().ok_or(unexpected_type("an object", value))?;
                let mut tuple = builder.start_tuple();
                for field in fields.iter() {
                    let value = object.get(&*field.name).ok_or(missing_field(&field.name))?;
                    field.ty.deserialize(value, tuple.as_builder())?;
                }
                tuple.end();
            }
        }
        Ok(())
    }
}

fn unexpected_type(expected: &'static str, value: &JsonValue) -> Error {
    let got = match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "a boolean",
        JsonValue::Number(_) => "a number",
        JsonValue::String(_) => "a string",
        JsonValue::Array(_) => "an array",
        JsonValue::Object(_) => "an object",
    };
    Error::UnexpectedType { expected, got }
}

fn missing_field(name: &str) -> Error {
    Error::MissingField { name: name.into() }
}
