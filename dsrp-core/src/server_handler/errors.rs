use std::fmt;
use failure::{Fail, Backtrace};
use server_handler::ClientId;
use messages::ChannelId;

#[derive(Debug)]
pub struct ClientMessageHandlingError {
    pub kind: ClientMessageHandlingErrorKind,
}

#[derive(Debug, Fail)]
pub enum ClientMessageHandlingErrorKind {
    #[fail(display = "Unknown client id: {:?}", _0)]
    UnknownClientId(ClientId),

    #[fail(display = "{:?} does not exist", _0)]
    ChannelNotFound(ChannelId),

    #[fail(display = "{:?} is not owned by {} but instead owned by {}", channel, requesting_client, owning_client)]
    ChannelNotOwnedByRequester {
        channel: ChannelId,
        requesting_client: ClientId,
        owning_client: ClientId,
    }
}

#[derive(Debug)]
pub struct NewConnectionError {
    pub kind: NewConnectionErrorKind,
}

#[derive(Debug, Fail)]
pub enum NewConnectionErrorKind {
    #[fail(display = "Unknown channel: {:?}", _0)]
    UnknownChannelId(ChannelId),

    #[fail(display = "Channel {:?} is not a TCP channel", _0)]
    ConnectionAddedToNonTcpChannel(ChannelId),

    #[fail(display = "Connection created for channel {:?} that's marked as unbound", _0)]
    ConnectionAddedToUnboundChannel(ChannelId),
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

impl fmt::Display for NewConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl Fail for NewConnectionError {
    fn cause(&self) -> Option<&Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}