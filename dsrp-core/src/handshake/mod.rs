mod handshake_request;
mod handshake_response;

pub use self::handshake_request::{HandshakeRequest, HandshakeRequestParseError, HandshakeRequestParseErrorsKind};
pub use self::handshake_response::{HandshakeResponse};

pub(crate) static CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const HANDSHAKE_REQUEST_PREFIX: &[u8; 5] = b"DSRPA";
const HANDSHAKE_RESPONSE_PREFIX: &[u8; 5] = b"DSRPB";
