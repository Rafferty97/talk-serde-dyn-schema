use crate::{
    flatbin::{self, Flatbin},
    ty::Ty,
    JsonValue,
};

pub fn serialize(ty: &Ty, value: &Flatbin) -> flatbin::Result<JsonValue> {
    ty.serialize(value)
}

impl Ty {
    pub fn serialize(&self, value: &Flatbin) -> flatbin::Result<JsonValue> {
        Ok(match self {
            Ty::Bool => value.read_bool()?.into(),
            Ty::U64 => value.read_u64()?.into(),
            Ty::I64 => value.read_i64()?.into(),
            Ty::F64 => value.read_f64()?.into(),
            Ty::Bytes => value.read_bytes()?.into(),
            Ty::String => value.read_str()?.into(),
            Ty::Array { inner } => value
                .read_array()?
                .iter()
                .map(|bytes| inner.serialize(bytes))
                .collect::<flatbin::Result<Vec<_>>>()?
                .into(),
            Ty::Struct { fields } => fields
                .iter()
                .zip(value.read_tuple(fields.len())?)
                .map(|(field, bytes)| Ok((field.name.to_string(), field.ty.serialize(bytes)?)))
                .collect::<flatbin::Result<serde_json::Map<_, _>>>()?
                .into(),
        })
    }
}
