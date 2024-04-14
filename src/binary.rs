use crate::varint::{self, read_varint, write_varint, VarInt};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct FlatbinBuf {
    data: Vec<u8>,
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Flatbin {
    data: [u8],
}

pub struct FlatbinBuilder {
    buffer: Vec<u8>,
    seqs: Vec<Seq>,
}

struct Seq {
    remain: usize,
    offset: usize,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("the serialized data is of unexpected length")]
    UnexpectedLength,
    #[error("the deserialized number could not fit into the requested type")]
    NumberTooLarge,
    #[error("unexpected end of input")]
    UnexpectedEOF,
    #[error("a varint was malformed")]
    BadVarint,
    #[error("a string was not valid UTF-8")]
    InvalidUTF8,
}

pub type Result<T> = std::result::Result<T, Error>;

impl core::ops::Deref for FlatbinBuf {
    type Target = Flatbin;

    fn deref(&self) -> &Flatbin {
        Flatbin::from_bytes(&self.data)
    }
}

impl FlatbinBuf {
    pub fn new() -> Self {
        Self { data: vec![] }
    }

    pub fn from_vec(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.data
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
        let count = read_varint(&mut data)? as usize;
        Ok(Sequence { count, data })
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
    type Item = Result<&'a Flatbin>;
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

    pub fn iter(&self) -> SequenceIter<'a> {
        self.into_iter()
    }
}

impl<'a> Iterator for SequenceIter<'a> {
    type Item = Result<&'a Flatbin>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.count {
            0 => None,
            1 => {
                self.count -= 1;
                Some(Ok(Flatbin::from_bytes(self.data)))
            }
            _ => {
                self.count -= 1;

                let len = match read_varint(&mut self.data) {
                    Ok(len) => len as usize,
                    Err(err) => return Some(Err(err.into())),
                };
                if len > self.data.len() {
                    return Some(Err(Error::UnexpectedEOF));
                }
                let (element, rest) = self.data.split_at(len);
                self.data = rest;

                Some(Ok(Flatbin::from_bytes(element)))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<'a> ExactSizeIterator for SequenceIter<'a> {}

impl FlatbinBuilder {
    pub fn new() -> Self {
        Self {
            buffer: vec![],
            seqs: vec![],
        }
    }

    pub fn finish(self) -> FlatbinBuf {
        FlatbinBuf { data: self.buffer }
    }

    pub fn write_bool(&mut self, value: bool) {
        self.write(&[value as u8]);
    }

    pub fn write_uint(&mut self, value: impl Into<u64>) {
        let bytes = value.into().to_le_bytes();
        let count = bytes.iter().rposition(|&b| b != 0).map(|c| c + 1).unwrap_or(0);
        self.write_bytes(&bytes[..count]);
    }

    pub fn write_int(&mut self, value: impl Into<i64>) {
        let value = value.into();
        let value = if value < 0 { !(value << 1) } else { value << 1 };
        self.write_uint(value as u64);
    }

    pub fn write_f32(&mut self, value: f32) {
        self.write(&value.to_le_bytes());
    }

    pub fn write_f64(&mut self, value: f64) {
        self.write(&value.to_le_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.write(bytes);
    }

    pub fn write_string(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    pub fn start_tuple(&mut self, count: usize) {
        self.seqs.push(Seq {
            offset: self.buffer.len(),
            remain: count,
        });
    }

    pub fn start_array(&mut self, count: usize) {
        self.seqs.push(Seq {
            offset: self.buffer.len(),
            remain: count,
        });
        write_varint(&mut self.buffer, count as u64);
    }

    pub fn end_seq(&mut self) {
        let seq = self.seqs.pop().expect("not in a sequence");
        if seq.remain != 0 {
            panic!("too few elements in sequence");
        }

        if let Some(header) = self.make_header(self.buffer.len() - seq.offset) {
            let header = header.iter().copied();
            self.buffer.splice(seq.offset..seq.offset, header);
        }
    }

    fn write(&mut self, data: &[u8]) {
        if let Some(header) = self.make_header(data.len()) {
            self.buffer.extend(header.as_bytes());
        }
        self.buffer.extend(data);
    }

    fn make_header(&mut self, len: usize) -> Option<VarInt> {
        let seq = self.seqs.last_mut()?;
        let header = match seq.remain {
            0 => panic!("too many elements in sequence"),
            1 => None,
            2.. => Some(VarInt::from_usize(len)),
        };
        seq.remain -= 1;
        header
    }
}

impl From<varint::VarIntError> for Error {
    fn from(err: varint::VarIntError) -> Self {
        match err {
            varint::VarIntError::TooLarge => Error::BadVarint,
            varint::VarIntError::UnexpectedEOF => Error::UnexpectedEOF,
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn roundtrip() {
//         let mut builder = FlatbinBuilder::new();
//         builder.start_array();
//         builder.write_bool(true);
//         builder.write_string("Hello world");
//         builder.start_tuple();
//         builder.write_bytes(&[4, 5, 6]);
//         builder.write_bool(false);
//         builder.end_seq();
//         builder.write_void();
//         builder.end_seq();
//         let bytes = builder.build();

//         let a = bytes.read_array().unwrap();
//         assert_eq!(a.len(), 4);
//         let mut a = a.iter();
//         assert_eq!(a.next().unwrap().read_bool().unwrap(), true);
//         assert_eq!(a.next().unwrap().read_string().unwrap(), "Hello world");
//         let b = a.next().unwrap().read_tuple(2).unwrap();
//         assert_eq!(b.len(), 2);
//         let mut b = b.iter();
//         assert_eq!(b.next().unwrap().read_bytes().unwrap(), &[4, 5, 6]);
//         assert_eq!(b.next().unwrap().read_bool().unwrap(), false);
//         a.next().unwrap().read_void().unwrap();
//     }
// }
