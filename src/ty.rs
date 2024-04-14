/// A type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ty {
    /// A boolean.
    Bool,
    /// A 64-bit unsigned integer.
    U64,
    /// A 64-bit signed integer.
    I64,
    /// A 64-bit float.
    F64,
    /// A sequence of bytes.
    Bytes,
    /// A UTF-8 string.
    String,
    /// A homogenous sequence of values.
    Array {
        /// The type of elements in the sequence.
        inner: Box<Ty>,
    },
    /// A structure containing named fields.
    Struct {
        /// The fields comprising the struct.
        fields: Box<[Field]>,
    },
}

/// A struct field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Field {
    /// The name of the field.
    pub name: Box<str>,
    /// The type of the field.
    pub ty: Ty,
}

#[macro_export]
macro_rules! array_def {
    ($ty:expr) => {
        Ty::Array { inner: $ty.into() }
    };
}

#[macro_export]
macro_rules! struct_def {
    ({
        // Comma-separated key-value pairs
        $($key:literal : $value:expr),*
        // Allows trailing commas
        $(,)?
    }) => {{
        let fields = vec![
            // Expand each key-value pair
            $(
                Field {
                    name: $key.into(),
                    ty: $value,
                }
            ),*
        ].into();
        Ty::Struct { fields }
    }};
}

// FIXME: impl Display for Ty?
