mod client_message;
mod server_message;

pub use self::server_message::{ServerMessage, RegistrationFailureCause};
pub use self::client_message::{ClientMessage};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct RequestId(pub(crate) u32);

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ChannelId(pub(crate) u32);

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ConnectionId(pub(crate) u32);

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Tcp,
    Udp,
}