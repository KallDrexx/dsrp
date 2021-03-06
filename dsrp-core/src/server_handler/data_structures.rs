use std::fmt;
use std::collections::HashSet;
use handshake::HandshakeResponse;
use messages::{ChannelId, ConnectionId, ServerMessage, ConnectionType, RequestId};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ClientId(pub(crate) u32);

#[derive(Debug)]
pub struct NewClient {
    pub id: ClientId,
    pub response: HandshakeResponse,
}

pub struct ActiveClient {
    pub channels: HashSet<ChannelId>,
}

pub struct ActiveChannel {
    pub port: u16,
    pub connection_type: ConnectionType,
    pub owner: ClientId,
    pub tcp_connections: HashSet<ConnectionId>,
    pub socket_has_been_bound: bool,
    pub registration_request: RequestId,
}

pub struct ActiveTcpConnection {
    pub owning_channel: ChannelId,
    pub owning_client: ClientId,
}

/// Represents the different type of operations that the server handler instructs the
/// server to perform
#[derive(Debug)]
pub enum ServerOperation {
    /// Instructs the server to listen for TCP connections on the specified port, and what
    /// channel the connections should have events raised on
    StartTcpOperations {
        port: u16,
        channel: ChannelId,
    },

    /// Instructs the server to disconnect all tcp connections on the specified port and remove
    /// any associated tcp listeners
    StopTcpOperations {
        port: u16,
    },

    /// Instructs the server to listen for UDP packets on the specified port, and what channel
    /// it should relay them to the server handler under
    StartUdpOperations {
        port: u16,
        channel: ChannelId,
    },

    /// Instructs the server to stop listening for UDP packets on the specified port
    StopUdpOperations {
        port: u16,
    },

    /// Instructs the server to disconnect the specified TCP connection.  This is usually caused
    /// by the dsrp client registering a disconnection event on its side.
    DisconnectConnection {
        connection: ConnectionId,
    },

    /// Instructs a server to send a DSRP server message to the specified client
    SendMessageToDsrpClient {
        client: ClientId,
        message: ServerMessage,
    },

    /// Instructs the server to send a specific payload across a channel (and connection if
    /// the channel is for a TCP channel).
    SendByteData {
        channel: ChannelId,
        connection: Option<ConnectionId>,
        data: Vec<u8>,
    },
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}