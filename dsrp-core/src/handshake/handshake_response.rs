use std::io;
use std::fmt;
use std::string::FromUtf8Error;
use failure::{Backtrace, Fail};
use super::{HANDSHAKE_RESPONSE_PREFIX};

#[derive(PartialEq, Debug)]
pub enum HandshakeResponse {
    Success,
    Failure{reason: String},
}

#[derive(Debug)]
pub struct HandshakeResponseGenerationError {
    pub kind: HandshakeResponseGenerationErrorKind,
}

#[derive(Debug, Fail)]
pub enum HandshakeResponseGenerationErrorKind {
    #[fail(display = "Failure message can not be larger than 127 bytes")]
    FailureMessageTooLong,
}

#[derive(Debug)]
pub struct HandshakeResponseParseError {
    pub kind: HandshakeResponseParseErrorKind,
}

#[derive(Debug, Fail)]
pub enum HandshakeResponseParseErrorKind {
    #[fail(display = "Not enough bytes for a complete handshake")]
    NotEnoughBytes,

    #[fail(display = "Invalid prefix")]
    InvalidPrefix,

    #[fail(display = "Invalid type marker byte: {}", _0)]
    InvalidMarkerByte(u8),

    #[fail(display = "_0")]
    Io(#[cause] io::Error),

    #[fail(display = "_0")]
    FromUtf8Error(#[cause] FromUtf8Error),
}

impl HandshakeResponse {
    pub fn into_bytes(self) -> Result<Vec<u8>, HandshakeResponseGenerationError> {
        let mut bytes = Vec::with_capacity(6);
        bytes.extend_from_slice(HANDSHAKE_RESPONSE_PREFIX);

        match self {
            HandshakeResponse::Success => {
                bytes.push(0b10000000);
            },

            HandshakeResponse::Failure {reason} => {
                if reason.len() >= 0b10000000 {
                    let kind = HandshakeResponseGenerationErrorKind::FailureMessageTooLong;
                    return Err(HandshakeResponseGenerationError{kind});
                }

                bytes.push(reason.len() as u8);
                let message_bytes = reason.into_bytes();
                bytes.extend_from_slice(&message_bytes[..]);
            },
        }

        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), HandshakeResponseParseError> {
        let handshake_length = HANDSHAKE_RESPONSE_PREFIX.len();
        if bytes.len() < handshake_length + 1 {
            let kind = HandshakeResponseParseErrorKind::NotEnoughBytes;
            return Err(HandshakeResponseParseError{kind});
        }

        if &bytes[..handshake_length] != &HANDSHAKE_RESPONSE_PREFIX[..] {
            let kind = HandshakeResponseParseErrorKind::InvalidPrefix;
            return Err(HandshakeResponseParseError{kind});
        }

        let response = match bytes[handshake_length] {
            // values above 128 are reserved
            x if x > 128 => {
                let kind = HandshakeResponseParseErrorKind::InvalidMarkerByte(x);
                return Err(HandshakeResponseParseError{kind});
            },

            // 128 signifies success
            128 => {
                (HandshakeResponse::Success, &bytes[handshake_length + 1..])
            },

            // values below 128 are considered failures, with the actual number
            // being the number of bytes for the reason message
            x if x < 128 => {
                let start_index = handshake_length + 1;
                let end_index = start_index + x as usize;

                if bytes.len() < end_index {
                    let kind = HandshakeResponseParseErrorKind::NotEnoughBytes;
                    return Err(HandshakeResponseParseError{kind});
                }

                let reason_bytes = &bytes[start_index..end_index];
                let string = String::from_utf8(reason_bytes.to_vec())?;
                let response = HandshakeResponse::Failure {reason: string};
                let remaining_bytes = &bytes[end_index..];
                (response, remaining_bytes)
            },

            _ => unreachable!(),
        };

        Ok(response)
    }
}

impl fmt::Display for HandshakeResponseParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl Fail for HandshakeResponseParseError {
    fn cause(&self) -> Option<&Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}

impl From<HandshakeResponseParseErrorKind> for HandshakeResponseParseError {
    fn from(kind: HandshakeResponseParseErrorKind) -> Self {
        HandshakeResponseParseError { kind }
    }
}

impl From<io::Error> for HandshakeResponseParseError {
    fn from(error: io::Error) -> Self {
        HandshakeResponseParseError { kind: HandshakeResponseParseErrorKind::Io(error) }
    }
}

impl From<FromUtf8Error> for HandshakeResponseParseError {
    fn from(error: FromUtf8Error) -> Self {
        HandshakeResponseParseError { kind: HandshakeResponseParseErrorKind::FromUtf8Error(error) }
    }
}

impl fmt::Display for HandshakeResponseGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl Fail for HandshakeResponseGenerationError {
    fn cause(&self) -> Option<&Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}

impl From<HandshakeResponseGenerationErrorKind> for HandshakeResponseGenerationError {
    fn from(kind: HandshakeResponseGenerationErrorKind) -> Self {
        HandshakeResponseGenerationError { kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_convert_success_response_into_bytes() {
        let response = HandshakeResponse::Success;
        let bytes = response.into_bytes().unwrap();

        let prefix_length = HANDSHAKE_RESPONSE_PREFIX.len();
        assert_eq!(bytes.len(), prefix_length + 1, "Unexpected number of bytes");
        assert_eq!(&bytes[..prefix_length], HANDSHAKE_RESPONSE_PREFIX, "Unexpected prefix");
        assert_eq!(bytes[prefix_length], 0b10000000, "Unexpected response value");
    }

    #[test]
    fn can_convert_failure_response_into_bytes() {
        let message = "some failure".to_owned();
        let response = HandshakeResponse::Failure{reason: message.clone()};
        let bytes = response.into_bytes().unwrap();

        let prefix_length = HANDSHAKE_RESPONSE_PREFIX.len();
        assert_eq!(bytes.len(), prefix_length + 1 + message.len(), "Unexpected number of bytes");
        assert_eq!(&bytes[..prefix_length], HANDSHAKE_RESPONSE_PREFIX, "Unexpected prefix");
        assert_eq!(bytes[prefix_length], message.len() as u8, "Unexpected message length specified");
        assert_eq!(&bytes[prefix_length + 1..], &message.into_bytes()[..], "Unexpected message bytes");
    }

    #[test]
    fn can_read_success_bytes() {
        let response = HandshakeResponse::Success;
        let bytes = response.into_bytes().unwrap();
        let (response, _) = HandshakeResponse::from_bytes(&bytes).unwrap();

        assert_eq!(response, HandshakeResponse::Success, "Unexpected response parsed");
    }

    #[test]
    fn can_read_failure_bytes() {
        let message = "test fail".to_owned();
        let response = HandshakeResponse::Failure {reason: message.clone()};
        let bytes = response.into_bytes().unwrap();
        let (response, _) = HandshakeResponse::from_bytes(&bytes).unwrap();

        assert_eq!(response, HandshakeResponse::Failure {reason: message.clone()}, "Unexpected response");
    }

    #[test]
    fn parse_process_returns_extra_bytes() {
        let message = "test fail".to_owned();
        let response = HandshakeResponse::Failure {reason: message.clone()};
        let mut bytes = response.into_bytes().unwrap();
        bytes.extend_from_slice(&[1, 2, 3]);

        let (response, extra_bytes) = HandshakeResponse::from_bytes(&bytes).unwrap();

        assert_eq!(response, HandshakeResponse::Failure {reason: message.clone()}, "Unexpected response");
        assert_eq!(&extra_bytes[..], &[1, 2, 3], "Unexpected extra bytes");
    }

    #[test]
    fn invalid_prefix_returns_error() {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(b"abcde1");

        let error = HandshakeResponse::from_bytes(&bytes).unwrap_err();
        match error.kind {
            HandshakeResponseParseErrorKind::InvalidPrefix => (),
            x => panic!("Unexpected error: {}", x),
        }
    }

    #[test]
    fn error_returned_when_not_enough_bytes_passed_in() {
        let message = "test fail".to_owned();
        let response = HandshakeResponse::Failure {reason: message.clone()};
        let bytes = response.into_bytes().unwrap();
        let error = HandshakeResponse::from_bytes(&bytes[..8]).unwrap_err();

        match error.kind {
            HandshakeResponseParseErrorKind::NotEnoughBytes => (),
            x => panic!("Unexpected error: {}", x),
        }
    }
}