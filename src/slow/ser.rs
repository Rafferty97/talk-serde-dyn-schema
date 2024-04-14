use crate::{
    flatbin::{self, Flatbin, Result},
    ty::Ty,
};

pub fn serialize(ty: &Ty, value: &Flatbin) -> Result<serde_json::Value> {
    Ok(match ty {
        Ty::Bool => value.read_bool()?.into(),
        Ty::U64 => value.read_u64()?.into(),
        Ty::I64 => value.read_i64()?.into(),
        Ty::F64 => value.read_f64()?.into(),
        Ty::Bytes => value.read_bytes()?.into(),
        Ty::String => value.read_str()?.into(),
        Ty::Array { inner } => value
            .read_array()?
            .iter()
            .map(|bytes| serialize(inner, bytes))
            .collect::<flatbin::Result<Vec<_>>>()?
            .into(),
        Ty::Struct { fields } => fields
            .iter()
            .zip(value.read_tuple(fields.len())?)
            .map(|(field, bytes)| Ok((field.name.to_string(), serialize(&field.ty, bytes)?)))
            .collect::<Result<serde_json::Map<_, _>>>()?
            .into(),
    })
}
