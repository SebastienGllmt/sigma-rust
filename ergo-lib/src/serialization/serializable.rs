//! Serialization of Ergo types
use super::{
    constant_store::ConstantStore,
    sigma_byte_reader::{SigmaByteRead, SigmaByteReader},
    sigma_byte_writer::{SigmaByteWrite, SigmaByteWriter},
};
use io::Cursor;
use sigma_ser::{peekable_reader::PeekableReader, vlq_encode};
use std::io;
use thiserror::Error;

/// Ways serialization might fail
#[derive(Error, Eq, PartialEq, Debug, Clone)]
pub enum SerializationError {
    /// Failed to parse op
    #[error("op parsing error")]
    InvalidOpCode,
    /// Lacking support for the op
    #[error("not implemented op error")]
    NotImplementedOpCode(u8),
    /// Failed to parse type
    #[error("type parsing error")]
    InvalidTypePrefix,
    /// Failed to decode VLQ
    #[error("vlq encode error")]
    VlqEncode(vlq_encode::VlqEncodingError),
    /// IO fail (EOF, etc.)
    #[error("io error")]
    Io(String),
    /// Misc fail
    #[error("misc error")]
    Misc(String),
    /// Feature not yet implemented
    #[error("feature not yet implemented: {0}")]
    NotImplementedYet(String),
    /// Constant with given index not found in constant store
    #[error("Constant with index {0} not found in constant store")]
    ConstantForPlaceholderNotFound(u32),
    /// Value out of bounds
    #[error("Value out of bounds: {0}")]
    ValueOutOfBounds(String),
}

impl From<vlq_encode::VlqEncodingError> for SerializationError {
    fn from(error: vlq_encode::VlqEncodingError) -> Self {
        SerializationError::VlqEncode(error)
    }
}

impl From<io::Error> for SerializationError {
    fn from(error: io::Error) -> Self {
        SerializationError::Io(error.to_string())
    }
}

/// Consensus-critical serialization for Ergo
pub trait SigmaSerializable: Sized {
    /// Write `self` to the given `writer`.
    /// This function has a `sigma_` prefix to alert the reader that the
    /// serialization in use is consensus-critical serialization    
    /// Notice that the error type is [`std::io::Error`]; this indicates that
    /// serialization MUST be infallible up to errors in the underlying writer.
    /// In other words, any type implementing `SigmaSerializable`
    /// must make illegal states unrepresentable.
    fn sigma_serialize<W: SigmaByteWrite>(&self, w: &mut W) -> Result<(), io::Error>;

    /// Try to read `self` from the given `reader`.
    /// `sigma-` prefix to alert the reader that the serialization in use
    /// is consensus-critical
    fn sigma_parse<R: SigmaByteRead>(r: &mut R) -> Result<Self, SerializationError>;

    /// Serialize any SigmaSerializable value into bytes
    fn sigma_serialize_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let mut w = SigmaByteWriter::new(&mut data, None);
        self.sigma_serialize(&mut w)
            // since serialization may fail only for underlying IO errors it's ok to force unwrap
            .expect("serialization failed");
        data
    }

    /// Parse `self` from the bytes
    fn sigma_parse_bytes(mut bytes: Vec<u8>) -> Result<Self, SerializationError> {
        let cursor = Cursor::new(&mut bytes[..]);
        let pr = PeekableReader::new(cursor);
        let mut sr = SigmaByteReader::new(pr, ConstantStore::empty());
        Self::sigma_parse(&mut sr)
    }
}

/// serialization roundtrip
#[cfg(test)]
pub fn sigma_serialize_roundtrip<T: SigmaSerializable>(v: &T) -> T {
    let mut data = Vec::new();
    let mut w = SigmaByteWriter::new(&mut data, None);
    v.sigma_serialize(&mut w).expect("serialization failed");
    let cursor = Cursor::new(&mut data[..]);
    let pr = PeekableReader::new(cursor);
    let mut sr = SigmaByteReader::new(pr, ConstantStore::empty());
    T::sigma_parse(&mut sr).expect("parse failed")
}
