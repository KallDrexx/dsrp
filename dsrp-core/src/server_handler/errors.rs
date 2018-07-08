use std::fmt;
use failure::{Fail, Backtrace};
use server_handler::ClientId;

#[derive(Debug)]
pub struct ClientMessageHandlingError {
    pub kind: ClientMessageHandlingErrorKind,
}

#[derive(Debug, Fail)]
pub enum ClientMessageHandlingErrorKind {
    #[fail(display = "Unknown client id: {}", _0)]
    UnknownClientId(ClientId),
}

impl fmt::Display for ClientMessageHandlingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl Fail for ClientMessageHandlingError {
    fn cause(&self) -> Option<&Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}