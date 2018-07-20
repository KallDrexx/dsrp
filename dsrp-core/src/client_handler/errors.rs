use std::fmt;
use failure::{Fail, Backtrace};
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

impl Fail for ServerMessageHandlingError {
    fn cause(&self) -> Option<&Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}