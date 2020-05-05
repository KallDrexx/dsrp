use std::fmt;
use failure::Fail;
use messages::RequestId;

#[derive(Debug)]
pub struct ServerMessageHandlingError {
    pub kind: ServerMessageHandlingErrorKind,
}

#[derive(Debug, Fail)]
pub enum ServerMessageHandlingErrorKind {
    #[fail(display = "Unknown request id: {:?}", _0)]
    UnknownRequest(RequestId),
}

impl fmt::Display for ServerMessageHandlingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}