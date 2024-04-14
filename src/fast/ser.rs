use crate::{
    flatbin::{self, Flatbin},
    ty::Ty,
};
use serde::{ser::SerializeMap, ser::SerializeSeq, Serialize, Serializer};

pub fn serialize<S: Serializer>(ser: S, ty: &Ty, value: &Flatbin) -> Result<S::Ok, S::Error> {
    match ty {
        Ty::Bool => ser.serialize_bool(value.read_bool().map_err(corrupt)?),
        Ty::U64 => ser.serialize_u64(value.read_u64().map_err(corrupt)?),
        Ty::I64 => ser.serialize_i64(value.read_i64().map_err(corrupt)?),
        Ty::F64 => ser.serialize_f64(value.read_f64().map_err(corrupt)?),
        Ty::Bytes => ser.serialize_bytes(value.read_bytes().map_err(corrupt)?),
        Ty::String => ser.serialize_str(value.read_str().map_err(corrupt)?),
        Ty::Array { inner } => {
            let array = value.read_array().map_err(corrupt)?;
            let mut seq = ser.serialize_seq(Some(array.len()))?;
            for value in array {
                let ctx = TypedValue { ty: inner, value };
                seq.serialize_element(&ctx)?;
            }
            seq.end()
        }
        Ty::Struct { fields } => {
            let tuple = value.read_tuple(fields.len()).map_err(corrupt)?;
            let mut map = ser.serialize_map(Some(fields.len()))?;
            for (field, value) in fields.iter().zip(tuple) {
                let ctx = TypedValue { ty: &field.ty, value };
                map.serialize_entry(&*field.name, &ctx)?;
            }
            map.end()
        }
    }
}

struct TypedValue<'a> {
    pub ty: &'a Ty,
    pub value: &'a Flatbin,
}

impl<'a> Serialize for TypedValue<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize(serializer, self.ty, self.value)
    }
}

fn corrupt<E: serde::ser::Error>(_: flatbin::Error) -> E {
    E::custom("corrupt document")
}
