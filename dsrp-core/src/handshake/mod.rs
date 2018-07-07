mod handshake_request;
mod handshake_response;

pub use self::handshake_request::{HandshakeRequest, HandshakeRequestParseError, HandshakeRequestParseErrorsKind};
pub use self::handshake_response::{HandshakeResponse};

pub(crate) const CURRENT_PROTOCOL_VERSION: u32 = 1;
const HANDSHAKE_REQUEST_PREFIX: &[u8; 5] = b"DSRPA";
const HANDSHAKE_RESPONSE_PREFIX: &[u8; 5] = b"DSRPB";
