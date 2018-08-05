use std::io;
use std::io::{Read};
use std::fmt;
use std::string::FromUtf8Error;
use byteorder::{ WriteBytesExt};
use failure::{Backtrace, Fail};
use super::{CURRENT_VERSION, HANDSHAKE_REQUEST_PREFIX};

pub struct HandshakeRequest {
    pub client_protocol_version: String,
}

#[derive(Debug)]
pub struct HandshakeRequestParseError {
    pub kind: HandshakeRequestParseErrorsKind,
}

#[derive(Debug, Fail)]
pub enum HandshakeRequestParseErrorsKind {
    #[fail(display = "Incorrect number of bytes")]
    InvalidNumberOfBytes,

    #[fail(display = "Invalid prefix")]
    InvalidPrefix,

    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),

    #[fail(display = "{}", _0)]
    FromUtf8Error(#[cause] FromUtf8Error),
}

impl HandshakeRequest {
    pub fn new() -> Self {
        HandshakeRequest {
            client_protocol_version: CURRENT_VERSION.to_owned(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(9);
        for byte in HANDSHAKE_REQUEST_PREFIX {
            bytes.push(*byte);
        }

        if self.client_protocol_version.len() > 255 {
            panic!("Handshake protocol version is {} characters, but it can't be more than 255", self.client_protocol_version.len());
        }

        bytes.write_u8(self.client_protocol_version.len() as u8).unwrap();
        bytes.extend_from_slice(self.client_protocol_version.as_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HandshakeRequestParseError> {
        let prefix_length = HANDSHAKE_REQUEST_PREFIX.len();
        if bytes.len() < prefix_length + 1 {
            let kind = HandshakeRequestParseErrorsKind::InvalidNumberOfBytes;
            return Err(HandshakeRequestParseError {kind});
        }

        let prefix = &bytes[..prefix_length];
        let version_length = bytes[prefix_length];

        if &prefix[..] != HANDSHAKE_REQUEST_PREFIX {
            let kind = HandshakeRequestParseErrorsKind::InvalidPrefix;
            return Err(HandshakeRequestParseError {kind});
        }

        let expected_length = prefix_length + 1 + (version_length as usize);
        if bytes.len() != expected_length {
            let kind = HandshakeRequestParseErrorsKind::InvalidNumberOfBytes;
            return Err(HandshakeRequestParseError {kind});
        }

        let start_index = prefix_length + 1;
        let mut buffer = Vec::with_capacity(version_length as usize);
        buffer.resize(version_length as usize, 0);
        (&bytes[start_index..]).read(&mut buffer)?;

        let value = String::from_utf8(buffer)?;
        Ok(HandshakeRequest{client_protocol_version: value})
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

impl From<FromUtf8Error> for HandshakeRequestParseError {
    fn from(error: FromUtf8Error) -> Self {
        HandshakeRequestParseError { kind: HandshakeRequestParseErrorsKind::FromUtf8Error(error) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use byteorder::{BigEndian, ReadBytesExt};

    #[test]
    fn can_convert_request_into_bytes() {
        const VERSION: &str = "12345";
        let request = HandshakeRequest { client_protocol_version: VERSION.to_owned() };
        let bytes = request.into_bytes();

        let prefix_length = HANDSHAKE_REQUEST_PREFIX.len();

        assert_eq!(bytes.len(), prefix_length + 1 + VERSION.len(), "Unexpected byte length");
        assert_eq!(&bytes[..prefix_length], HANDSHAKE_REQUEST_PREFIX, "Unexpected handshake prefix");

        let mut cursor = Cursor::new(&bytes[prefix_length..]);
        let version_length = cursor.read_u8().unwrap() as usize;
        assert_eq!(version_length, VERSION.len(), "Unexpected version string length");

        let mut buffer = Vec::new();
        buffer.resize(version_length as usize, 0);
        cursor.read_exact(&mut buffer[..]).unwrap();
        assert_eq!(&buffer[..], VERSION.as_bytes(), "Unexpected protocol version");
    }

    #[test]
    fn new_request_has_current_handshake_version() {
        let request = HandshakeRequest::new();

        assert_eq!(request.client_protocol_version, CURRENT_VERSION, "Unexpected protocol version");
    }

    #[test]
    fn can_read_deserialized_request() {
        const VERSION: &'static str = "abcdefg";
        let request = HandshakeRequest { client_protocol_version: VERSION.to_owned() };
        let bytes = request.into_bytes();
        let request = HandshakeRequest::from_bytes(&bytes).unwrap();

        assert_eq!(request.client_protocol_version, VERSION, "Unexpected client version");
    }

    #[test]
    fn invalid_prefix_returns_error() {
        let mut bytes = Vec::with_capacity(8);
        for byte in b"abcde" {
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