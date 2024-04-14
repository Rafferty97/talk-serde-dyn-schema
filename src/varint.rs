use std::ops::Deref;
use thiserror::Error;

/// A variable-length integer
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarInt {
    /// The number of bytes in the varint
    len: u8,
    /// The bytes of the varint, padded with zeros
    data: [u8; 10],
}

#[derive(Clone, Copy, Error, Debug)]
pub enum VarIntError {
    #[error("the varint is too large")]
    TooLarge,
    #[error("the varint was incomplete")]
    UnexpectedEOF,
}

pub fn read_varint(buffer: &mut &[u8]) -> Result<u64, VarIntError> {
    let value = VarInt::from_slice(buffer)?;
    *buffer = &buffer[value.num_bytes()..];
    Ok(value.as_u64())
}

pub fn write_varint(buffer: &mut Vec<u8>, value: u64) {
    buffer.extend(VarInt::from_u64(value).as_bytes());
}

impl VarInt {
    pub fn from_slice(buffer: &[u8]) -> Result<Self, VarIntError> {
        let err = || {
            if buffer.len() > 10 {
                VarIntError::TooLarge
            } else {
                VarIntError::UnexpectedEOF
            }
        };

        let len = buffer[..buffer.len().min(10)]
            .iter()
            .position(|b| *b & 0x80 == 0)
            .map(|index| index + 1)
            .ok_or_else(err)?;

        let mut data = [0; 10];
        data[..len].copy_from_slice(&buffer[..len]);
        let len = len as u8;

        Ok(Self { len, data })
    }

    pub fn from_u8(value: u8) -> Self {
        Self::from_u64(value as u64)
    }

    pub fn from_u16(value: u16) -> Self {
        Self::from_u64(value as u64)
    }

    pub fn from_u32(value: u32) -> Self {
        Self::from_u64(value as u64)
    }

    pub fn from_u64(mut value: u64) -> Self {
        let mut data = [0; 10];
        for i in 0..10 {
            let byte = value as u8;
            value >>= 7;
            if value > 0 {
                data[i] = byte | 0x80;
            } else {
                data[i] = byte;
                let len = (i + 1) as u8;
                return Self { len, data };
            }
        }
        unreachable!()
    }

    pub fn from_usize(value: usize) -> Self {
        Self::from_u64(value as u64)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..(self.len as usize)]
    }

    pub fn as_u64(&self) -> u64 {
        self.data
            .iter()
            .take(self.len as usize)
            .enumerate()
            .map(|(index, byte)| ((*byte as u64) & 0x7f) << (7 * index))
            .sum()
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.as_u64().try_into().ok()
    }

    pub fn num_bytes(&self) -> usize {
        self.len as usize
    }
}

impl Deref for VarInt {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn small_varints() {
        for value in 0..100_000 {
            let a = VarInt::from_u64(value);
            assert_eq!(a.as_u64(), value);
            assert_eq!(VarInt::from_slice(a.as_bytes()).unwrap().as_u64(), value);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn large_varints() {
        for divisor in [1, 10, 100, 1000, 10_1000, 100_000] {
            let value = u64::MAX / divisor;
            let a = VarInt::from_u64(value);
            assert_eq!(a.as_u64(), value);
            assert_eq!(VarInt::from_slice(a.as_bytes()).unwrap().as_u64(), value);
        }
    }
}
