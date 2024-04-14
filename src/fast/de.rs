use crate::{
    flatbin::{Builder, FlatbinBuf},
    ty::{Field, Ty},
};
use serde::{
    de::{DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserializer,
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

// pub type Result<T> = std::result::Result<T, Error>;

pub fn deserialize(ty: &Ty, value: &str) -> serde_json::Result<FlatbinBuf> {
    let mut buffer = FlatbinBuf::new();
    deserialize_into(ty, value, &mut buffer)?;
    Ok(buffer)
}

pub fn deserialize_into(ty: &Ty, value: &str, buffer: &mut FlatbinBuf) -> serde_json::Result<()> {
    let mut de = serde_json::Deserializer::from_str(value);
    let builder = Builder::new(buffer);
    deserialize_value(&mut de, ty, builder)?;
    Ok(())
}

fn deserialize_value<'de, D: Deserializer<'de>>(de: D, ty: &Ty, builder: Builder) -> Result<(), D::Error> {
    match ty {
        Ty::Bool => de.deserialize_bool(BoolVisitor { builder }),
        Ty::U64 => de.deserialize_u64(UIntVisitor { builder }),
        Ty::I64 => de.deserialize_i64(IntVisitor { builder }),
        Ty::F64 => de.deserialize_f64(FloatVisitor { builder }),
        Ty::Bytes => de.deserialize_bytes(BytesVisitor { builder }),
        Ty::String => de.deserialize_str(StringVisitor { builder }),
        Ty::Array { inner } => de.deserialize_seq(ArrayVisitor { inner, builder }),
        Ty::Struct { fields } => de.deserialize_map(StructVisitor { fields, builder }),
    }
}

struct TypedBuilder<'a> {
    pub ty: &'a Ty,
    pub builder: Builder<'a>,
}

impl<'de, 'a> DeserializeSeed<'de> for TypedBuilder<'a> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserialize_value(deserializer, self.ty, self.builder)
    }
}

struct BoolVisitor<'a> {
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for BoolVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a boolean")
    }

    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<(), E> {
        self.builder.write_bool(value);
        Ok(())
    }
}

const OUT_OF_RANGE: &str = "value is outside numeric range for type";

struct UIntVisitor<'a> {
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for UIntVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a non-negative integer")
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<(), E> {
        self.builder.write_u64(value);
        Ok(())
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<(), E> {
        let value = u64::try_from(value).map_err(|_| E::custom(OUT_OF_RANGE))?;
        self.builder.write_u64(value);
        Ok(())
    }
}

struct IntVisitor<'a> {
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for IntVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an integer")
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<(), E> {
        let value = i64::try_from(value).map_err(|_| E::custom(OUT_OF_RANGE))?;
        self.builder.write_i64(value);
        Ok(())
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<(), E> {
        self.builder.write_i64(value);
        Ok(())
    }
}

struct FloatVisitor<'a> {
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for FloatVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a number")
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<(), E> {
        let value = value as f64;
        if value.is_infinite() {
            return Err(E::custom(OUT_OF_RANGE));
        }
        self.builder.write_f64(value);
        Ok(())
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<(), E> {
        let value = value as f64;
        if value.is_infinite() {
            return Err(E::custom(OUT_OF_RANGE));
        }
        self.builder.write_f64(value);
        Ok(())
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<(), E> {
        self.builder.write_f64(value);
        Ok(())
    }
}

struct BytesVisitor<'a> {
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for BytesVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a byte array")
    }

    fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<(), E> {
        self.builder.write_bytes(value);
        Ok(())
    }
}

struct StringVisitor<'a> {
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for StringVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a string")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<(), E> {
        self.builder.write_str(value);
        Ok(())
    }
}

struct ArrayVisitor<'a> {
    pub inner: &'a Ty,
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for ArrayVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an array")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let mut vector = self.builder.start_vector();
        loop {
            let ctx = TypedBuilder {
                ty: self.inner,
                builder: vector.as_builder(),
            };
            if seq.next_element_seed(ctx)?.is_none() {
                break;
            }
        }
        vector.end();
        Ok(())
    }
}

struct StructVisitor<'a> {
    pub fields: &'a [Field],
    pub builder: Builder<'a>,
}

impl<'a, 'de> Visitor<'de> for StructVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut tuple = self.builder.start_tuple();
        while let Some(key) = map.next_key::<&str>()? {
            let ctx = TypedBuilder {
                ty: &self.fields.iter().find(|f| &*f.name == key).unwrap().ty,
                builder: tuple.as_builder(),
            };
            map.next_value_seed(ctx)?;
        }
        tuple.end();
        Ok(())
    }
}
