mod client_message;
mod server_message;

pub use self::server_message::{ServerMessage, RegistrationFailureCause};
pub use self::client_message::{ClientMessage};

pub struct RequestId(u32);
pub struct ChannelId(u32);
pub struct ConnectionId(u32);

pub enum ConnectionType {
    Tcp,
    Udp,
}