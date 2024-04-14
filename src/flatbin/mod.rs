pub use builder::*;
use std::hint::unreachable_unchecked;
use thiserror::Error;

mod builder;
mod util;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the serialized data is of unexpected length")]
    UnexpectedLength,
    #[error("the deserialized number could not fit into the requested type")]
    NumberTooLarge,
    #[error("unexpected end of input")]
    UnexpectedEOF,
    #[error("a string was not valid UTF-8")]
    InvalidUTF8,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Default)]
pub struct FlatbinBuf {
    data: Vec<u8>,
}

impl FlatbinBuf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

#[repr(transparent)]
pub struct Flatbin {
    data: [u8],
}

impl core::ops::Deref for FlatbinBuf {
    type Target = Flatbin;

    fn deref(&self) -> &Flatbin {
        Flatbin::from_bytes(&self.data)
    }
}

impl Flatbin {
    pub fn from_bytes(bytes: &[u8]) -> &Self {
        // SAFETY: `Flatdata` has the same layout as `[u8]` via #[repr(transparent)].
        unsafe { std::mem::transmute(bytes) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn read_void(&self) -> Result<()> {
        if self.data.is_empty() {
            Ok(())
        } else {
            Err(Error::UnexpectedLength)
        }
    }

    pub fn read_bool(&self) -> Result<bool> {
        match self.read_uint()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::NumberTooLarge),
        }
    }

    pub fn read_u8(&self) -> Result<u8> {
        self.read_uint()?.try_into().map_err(|_| Error::NumberTooLarge)
    }

    pub fn read_u16(&self) -> Result<u16> {
        self.read_uint()?.try_into().map_err(|_| Error::NumberTooLarge)
    }

    pub fn read_u32(&self) -> Result<u32> {
        self.read_uint()?.try_into().map_err(|_| Error::NumberTooLarge)
    }

    pub fn read_u64(&self) -> Result<u64> {
        self.read_uint()
    }

    pub fn read_uint(&self) -> Result<u64> {
        if self.data.len() > 8 {
            return Err(Error::UnexpectedLength);
        }

        let mut bytes = [0; 8];
        bytes[..self.data.len()].copy_from_slice(&self.data);

        Ok(u64::from_le_bytes(bytes))
    }

    pub fn read_i8(&self) -> Result<i8> {
        self.read_int()?.try_into().map_err(|_| Error::NumberTooLarge)
    }

    pub fn read_i16(&self) -> Result<i16> {
        self.read_int()?.try_into().map_err(|_| Error::NumberTooLarge)
    }

    pub fn read_i32(&self) -> Result<i32> {
        self.read_int()?.try_into().map_err(|_| Error::NumberTooLarge)
    }

    pub fn read_i64(&self) -> Result<i64> {
        self.read_int()
    }

    pub fn read_int(&self) -> Result<i64> {
        let value = self.read_uint()?;
        if value & 1 == 0 {
            Ok((value >> 1) as _)
        } else {
            Ok(!(value >> 1) as _)
        }
    }

    pub fn read_f32(&self) -> Result<f32> {
        if let [a, b, c, d] = &self.data {
            Ok(f32::from_le_bytes([*a, *b, *c, *d]))
        } else {
            Err(Error::UnexpectedLength)
        }
    }

    pub fn read_f64(&self) -> Result<f64> {
        if let [a, b, c, d, e, f, g, h] = &self.data {
            Ok(f64::from_le_bytes([*a, *b, *c, *d, *e, *f, *g, *h]))
        } else {
            Err(Error::UnexpectedLength)
        }
    }

    pub fn read_bytes(&self) -> Result<&[u8]> {
        Ok(&self.data)
    }

    pub fn read_string(&self) -> Result<&str> {
        std::str::from_utf8(&self.data).map_err(|_| Error::InvalidUTF8)
    }

    pub fn read_tuple(&self, count: usize) -> Result<Sequence<'_>> {
        let data = &self.data;
        Ok(Sequence { count, data })
    }

    pub fn read_array(&self) -> Result<Sequence<'_>> {
        let mut data = &self.data;
        let count = if data.is_empty() {
            0
        } else {
            Self::read_varint(&mut data)? as usize
        };
        Ok(Sequence { count, data })
    }

    pub fn seek(&self, offset: usize) -> &Flatbin {
        let data = &self.data[offset..];
        let (header_len, body_len) = Flatbin::read_node_header(data).unwrap();
        Flatbin::from_bytes(&data[header_len..][..body_len])
    }

    fn read_varint(data: &mut &[u8]) -> Result<u64> {
        let mut value = 0;
        let mut index = 0;
        loop {
            let byte = data.get(index).ok_or(Error::UnexpectedEOF)?;
            value |= ((byte & 0x7f) as u64) << (7 * index);
            index += 1;
            if byte & 0x80 == 0 {
                *data = &data[index..];
                return Ok(value);
            }
        }
    }

    fn read_node_header(buffer: &[u8]) -> Result<(usize, usize)> {
        fn inner<const N: usize>(buffer: &[u8]) -> Result<(usize, usize)> {
            let mut bytes = [0; 8];
            bytes[..N].copy_from_slice(buffer.get(..N).ok_or(Error::UnexpectedEOF)?);
            bytes[0] <<= N + 1;
            Ok((N, (u64::from_le_bytes(bytes) >> (N + 1)) as usize))
        }

        fn inner2<const N: usize>(buffer: &[u8]) -> Result<(usize, usize)> {
            let mut bytes = [0; 8];
            bytes[..N].copy_from_slice(buffer.get(1..(N + 1)).ok_or(Error::UnexpectedEOF)?);
            Ok((N + 1, u64::from_le_bytes(bytes) as usize))
        }

        let first_byte = buffer.first().ok_or(Error::UnexpectedEOF)?;
        match first_byte.leading_ones() {
            // 1-byte literal
            0 => Ok((0, 1)),
            // 1-byte header
            1 => Ok((1, (first_byte & 0x3f) as _)),
            // Multi-byte headers
            2 => inner::<2>(buffer),
            3 => inner::<3>(buffer),
            4 => inner::<4>(buffer),
            5 => inner::<5>(buffer),
            6 => inner::<6>(buffer),
            7 => inner2::<7>(buffer),
            8 => inner2::<8>(buffer),
            // SAFETY: A `u8` cannot have more than 8 ones
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Sequence<'a> {
    count: usize,
    data: &'a [u8],
}

pub struct SequenceIter<'a> {
    count: usize,
    data: &'a [u8],
}

impl<'a> IntoIterator for Sequence<'a> {
    type Item = &'a Flatbin;
    type IntoIter = SequenceIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let Sequence { count, data } = self;
        SequenceIter { count, data }
    }
}

impl<'a> Sequence<'a> {
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> SequenceIter<'a> {
        self.into_iter()
    }
}

impl<'a> Iterator for SequenceIter<'a> {
    type Item = &'a Flatbin;

    fn next(&mut self) -> Option<Self::Item> {
        match self.count {
            0 => None,
            1 => {
                self.count -= 1;
                Some(Flatbin::from_bytes(self.data))
            }
            _ => {
                // If `read_node_header` encounters an error, we just return an empty slice,
                // effectively deferring the error until the value is actually read.
                let (header_len, body_len) = Flatbin::read_node_header(self.data).unwrap_or((0, 0));
                let (item, rest) = self.data[header_len..].split_at(body_len);

                self.data = rest;
                self.count -= 1;

                Some(Flatbin::from_bytes(item))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<'a> ExactSizeIterator for SequenceIter<'a> {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut buffer = FlatbinBuf::new();
        let builder = Builder::new(&mut buffer);
        let mut vec = builder.start_vector();
        vec.as_builder().write_bool(true);
        vec.as_builder().write_str("Hello world");
        let mut tup = vec.start_tuple();
        tup.as_builder().write_bytes(&[4, 5, 6]);
        tup.as_builder().write_bool(false);
        tup.end();
        vec.as_builder().write_void();
        vec.end();

        let a = buffer.read_array().unwrap();
        assert_eq!(a.len(), 4);
        let mut a = a.iter();
        assert_eq!(a.next().unwrap().read_bool().unwrap(), true);
        assert_eq!(a.next().unwrap().read_string().unwrap(), "Hello world");
        let b = a.next().unwrap().read_tuple(2).unwrap();
        assert_eq!(b.len(), 2);
        let mut b = b.iter();
        assert_eq!(b.next().unwrap().read_bytes().unwrap(), &[4, 5, 6]);
        assert_eq!(b.next().unwrap().read_bool().unwrap(), false);
        a.next().unwrap().read_void().unwrap();
    }
}
