use std::io;
use std::fmt;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::{Backtrace, Fail};
use super::{CURRENT_PROTOCOL_VERSION, HANDSHAKE_PREFIX};

const EXPECTED_REQUEST_LENGTH: usize = 8;

pub struct HandshakeRequest {
    pub client_protocol_version: u32,
}

#[derive(Debug)]
pub struct HandshakeRequestParseError {
    /// The type of error that was observed
    pub kind: HandshakeRequestParseErrorsKind,
}

#[derive(Debug, Fail)]
pub enum HandshakeRequestParseErrorsKind {
    #[fail(display = "Incorrect number of bytes")]
    InvalidNumberOfBytes,

    #[fail(display = "Invalid prefix")]
    InvalidPrefix,

    #[fail(display = "_0")]
    Io(#[cause] io::Error)
}

impl HandshakeRequest {
    pub fn new() -> Self {
        HandshakeRequest {
            client_protocol_version: CURRENT_PROTOCOL_VERSION,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        for byte in HANDSHAKE_PREFIX {
            bytes.push(*byte);
        }

        bytes.write_u32::<BigEndian>(self.client_protocol_version).unwrap();
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HandshakeRequestParseError> {
        let prefix_length = HANDSHAKE_PREFIX.len();
        if bytes.len() != EXPECTED_REQUEST_LENGTH {
            let kind = HandshakeRequestParseErrorsKind::InvalidNumberOfBytes;
            return Err(HandshakeRequestParseError {kind});
        }

        if &bytes[..prefix_length] != HANDSHAKE_PREFIX {
            let kind = HandshakeRequestParseErrorsKind::InvalidPrefix;
            return Err(HandshakeRequestParseError {kind});
        }

        let version = (&bytes[prefix_length..]).read_u32::<BigEndian>()?;
        return Ok(HandshakeRequest{
            client_protocol_version: version,
        })
    }
}

impl fmt::Display for HandshakeRequestParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl Fail for HandshakeRequestParseError {
    fn cause(&self) -> Option<&Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}

impl From<HandshakeRequestParseErrorsKind> for HandshakeRequestParseError {
    fn from(kind: HandshakeRequestParseErrorsKind) -> Self {
        HandshakeRequestParseError { kind }
    }
}

impl From<io::Error> for HandshakeRequestParseError {
    fn from(error: io::Error) -> Self {
        HandshakeRequestParseError { kind: HandshakeRequestParseErrorsKind::Io(error) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn can_convert_request_into_bytes() {
        const VERSION: u32 = 322;
        let request = HandshakeRequest { client_protocol_version: VERSION };
        let bytes = request.into_bytes();

        let prefix_length = HANDSHAKE_PREFIX.len();

        assert_eq!(bytes.len(), prefix_length + 4, "Unexpected byte length");
        assert_eq!(&bytes[..prefix_length], HANDSHAKE_PREFIX, "Unexpected handshake prefix");

        let mut cursor = Cursor::new(&bytes[prefix_length..]);
        let actual_version = cursor.read_u32::<BigEndian>().unwrap();
        assert_eq!(actual_version, VERSION, "Unexpected protocol version");
    }

    #[test]
    fn new_request_has_current_handshake_version() {
        let request = HandshakeRequest::new();

        assert_eq!(request.client_protocol_version, CURRENT_PROTOCOL_VERSION, "Unexpected protocol version");
    }

    #[test]
    fn can_read_deserialized_request() {
        const VERSION: u32 = 322;
        let request = HandshakeRequest { client_protocol_version: VERSION };
        let bytes = request.into_bytes();
        let request = HandshakeRequest::from_bytes(&bytes).unwrap();

        assert_eq!(request.client_protocol_version, VERSION, "Unexpected client version");
    }

    #[test]
    fn invalid_prefix_returns_error() {
        let mut bytes = Vec::with_capacity(8);
        for byte in b"abcd" {
            bytes.push(*byte);
        }
        bytes.write_u32::<BigEndian>(15).unwrap();

        match HandshakeRequest::from_bytes(&bytes) {
            Err(HandshakeRequestParseError{kind: HandshakeRequestParseErrorsKind::InvalidPrefix})
                => (), // success

            Ok(_) => panic!("Expected error, received OK()"),
            Err(x) => panic!("Expected invalid prefix error, received {}", x),
        }
    }
}