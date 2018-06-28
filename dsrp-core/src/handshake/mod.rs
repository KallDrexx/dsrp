mod handshake_request;
mod handshake_response;

pub use self::handshake_request::{HandshakeRequest, HandshakeRequestParseError, HandshakeRequestParseErrorsKind};
pub use self::handshake_response::{HandshakeResponse};

const CURRENT_PROTOCOL_VERSION: u32 = 1;
const HANDSHAKE_PREFIX: &[u8; 4] = b"DSRP";