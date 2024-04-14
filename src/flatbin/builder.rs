use super::{util::VarInt, Flatbin, FlatbinBuf};
use arrayvec::ArrayVec;

pub struct Builder<'a> {
    buffer: &'a mut Vec<u8>,
    last_child: Option<&'a mut Option<usize>>,
    count: Option<&'a mut usize>,
}

pub struct TupleBuilder<'a> {
    last_child: Option<usize>,
    buffer: &'a mut Vec<u8>,
}

pub struct VectorBuilder<'a> {
    start: usize,
    count: usize,
    last_child: Option<usize>,
    buffer: &'a mut Vec<u8>,
}

impl<'a> Builder<'a> {
    pub fn new(buffer: &'a mut FlatbinBuf) -> Self {
        Self {
            buffer: &mut buffer.data,
            last_child: None,
            count: None,
        }
    }

    pub fn write<T: Writable>(self, value: T) {
        value.write(self)
    }

    pub fn write_void(mut self) {
        self.begin_write();
    }

    pub fn write_bool(self, value: bool) {
        match value {
            true => self.write_u8(1),
            false => self.write_u8(0),
        }
    }

    pub fn write_u8(mut self, value: u8) {
        self.begin_write();
        match value {
            0..=0x7f => self.buffer.push(value),
            _ => self.buffer.extend([129, value]),
        }
    }

    pub fn write_u16(self, value: u16) {
        self.write_u64(value.into())
    }

    pub fn write_u32(self, value: u32) {
        self.write_u64(value.into())
    }

    pub fn write_u64(mut self, value: u64) {
        self.begin_write();
        let count = (71 - value.leading_zeros() as usize) / 8;
        let bytes = value.to_le_bytes();
        self.buffer.extend(&bytes[..count]);
    }

    pub fn write_i64(self, value: i64) {
        let value = if value < 0 { !(value << 1) } else { value << 1 };
        self.write_u64(value as u64)
    }

    pub fn write_f32(self, value: f32) {
        self.write_bytes(&value.to_le_bytes())
    }

    pub fn write_f64(self, value: f64) {
        self.write_bytes(&value.to_le_bytes())
    }

    pub fn write_bytes(mut self, bytes: &[u8]) {
        self.begin_write();
        self.buffer.extend(bytes);
    }

    pub fn write_str(self, str: &str) {
        self.write_bytes(str.as_bytes())
    }

    pub fn copy(self, other: &Flatbin) {
        self.write_bytes(other.as_bytes())
    }

    pub fn start_tuple(mut self) -> TupleBuilder<'a> {
        self.begin_write();
        TupleBuilder::new(self.buffer)
    }

    pub fn start_vector(mut self) -> VectorBuilder<'a> {
        self.begin_write();
        VectorBuilder::new(self.buffer)
    }

    fn begin_write(&mut self) {
        if let Some(last_child) = self.last_child.take() {
            if let Some(offset) = *last_child {
                let header = make_header(&self.buffer[offset..]);
                self.buffer.splice(offset..offset, header);
            }
            *last_child = Some(self.buffer.len());
        }
        if let Some(count) = self.count.take() {
            *count += 1;
        }
    }
}

impl<'a> TupleBuilder<'a> {
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        TupleBuilder {
            last_child: None,
            buffer,
        }
    }

    pub fn as_builder(&mut self) -> Builder<'_> {
        Builder {
            buffer: self.buffer,
            last_child: Some(&mut self.last_child),
            count: None,
        }
    }

    pub fn write<T: Writable>(&mut self, value: T) {
        self.as_builder().write(value)
    }

    pub fn start_tuple(&mut self) -> TupleBuilder<'_> {
        self.as_builder().start_tuple()
    }

    pub fn start_vector(&mut self) -> VectorBuilder<'_> {
        self.as_builder().start_vector()
    }

    pub fn end(self) {}
}

impl<'a> VectorBuilder<'a> {
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        VectorBuilder {
            start: buffer.len(),
            count: 0,
            last_child: None,
            buffer,
        }
    }

    pub fn as_builder(&mut self) -> Builder<'_> {
        Builder {
            buffer: self.buffer,
            last_child: Some(&mut self.last_child),
            count: Some(&mut self.count),
        }
    }

    pub fn write<T: Writable>(&mut self, value: T) {
        self.as_builder().write(value)
    }

    pub fn start_tuple(&mut self) -> TupleBuilder<'_> {
        self.as_builder().start_tuple()
    }

    pub fn start_vector(&mut self) -> VectorBuilder<'_> {
        self.as_builder().start_vector()
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn end(self) -> usize {
        self.count
    }
}

impl Drop for VectorBuilder<'_> {
    fn drop(&mut self) {
        if self.count > 0 {
            let start = self.start;
            let count = VarInt::from_usize(self.count);
            self.buffer.splice(start..start, count.iter().copied());
        }
    }
}

fn make_header(body: &[u8]) -> ArrayVec<u8, 10> {
    match body {
        // Empty body
        [] => [0x80].into_iter().collect(),
        // 7-bit byte optimisation
        [byte] if *byte < 0x80 => [].into_iter().collect(),
        // 1 byte body
        [_byte] => [0x81].into_iter().collect(),
        // Multi-byte body
        _ => {
            let len = body.len();
            let count = ((71 - len.leading_zeros()) / 7) as usize;
            if count > 6 {
                let mut out = ArrayVec::new();
                out.push(0xff);
                out.extend(u64::to_le_bytes(len as u64));
                out
            } else {
                let mut bytes = u64::to_le_bytes((len as u64) << (count + 1));
                bytes[0] >>= count + 1;
                bytes[0] |= !(!0 >> count);
                bytes[..count].iter().copied().collect()
            }
        }
    }
}

pub trait Writable {
    fn write(self, builder: Builder);
}

macro_rules! impl_writable {
    ($type:ident, $method:ident) => {
        impl Writable for $type {
            fn write(self, builder: Builder) {
                builder.$method(self as _)
            }
        }
    };
}

impl_writable!(u8, write_u8);
impl_writable!(u16, write_u64);
impl_writable!(u32, write_u64);
impl_writable!(u64, write_u64);
impl_writable!(usize, write_u64);
impl_writable!(i8, write_i64);
impl_writable!(i16, write_i64);
impl_writable!(i32, write_i64);
impl_writable!(i64, write_i64);
impl_writable!(isize, write_u64);
impl_writable!(f32, write_f32);
impl_writable!(f64, write_f64);

impl Writable for &[u8] {
    fn write(self, builder: Builder) {
        builder.write_bytes(self)
    }
}

impl Writable for &str {
    fn write(self, builder: Builder) {
        builder.write_str(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_builder() {
        let mut buffer = FlatbinBuf::new();
        let builder = Builder::new(&mut buffer);
        let mut vec = builder.start_vector();
        vec.as_builder().write_u32(56);
        let mut vec2 = vec.start_vector();
        vec2.as_builder().write_u32(30);
        vec2.as_builder().write_u32(60);
        vec2.end();
        let vec2 = vec.start_vector();
        vec2.end();
        let mut tup = vec.start_tuple();
        tup.as_builder().write_u32(40);
        tup.as_builder().write_str("Hello");
        tup.as_builder().write_u32(50);
        tup.end();
        vec.as_builder().write_u32(12_899);
        vec.end();

        assert_eq!(
            &buffer.data[..],
            [
                5,       // Vector length = 4
                56,      // 7-bit optimised uint
                128 + 3, // Node size = 3
                2,       // Vector length = 2
                30,      // 7-bit optimised uint
                60,      // 7-bit optimised uint
                128,     // Node size = 0 (empty vector)
                128 + 8, // Node size = 8
                40,      // 7-bit optimised uint
                128 + 5, // Node size = 5
                b'H',
                b'e',
                b'l',
                b'l',
                b'o',
                50, // 7-bit optimised uint
                // Last element, node size elided
                (12_899 % 256) as u8, // LSB
                (12_899 / 256) as u8, // MSB
            ]
        )
    }
}
