use std::collections::HashSet;
use messages::{ClientMessage, ConnectionType, RequestId, ChannelId, ConnectionId};
use messages::RegistrationFailureCause;

#[derive(Debug)]
pub enum OutstandingRequest {
    Registration{
        connection_type: ConnectionType,
        port: u16,
    }
}

#[derive(Debug, PartialEq)]
pub struct ActiveChannel {
    pub connection_type: ConnectionType,
    pub connections: HashSet<ConnectionId>,
}

pub struct ActiveConnection {
    pub owner: ChannelId,
}

#[derive(Debug)]
pub enum ClientOperation {
    /// Notifies the client that a port registration request was successful and a channel
    /// was assigned for communication over it.
    NotifyChannelOpened {
        registered_by_request: RequestId,
        opened_channel: ChannelId,
    },

    /// The specified message should be sent to the DSRP server the client is connected to
    SendMessageToServer {
        message: ClientMessage,
    },

    /// Notifies the client that the DSRP server is reporting a new remote inbound connection
    /// has been accepted for the specified channel, and instructs the client to create it's
    /// own matching TCP connection from the DSRP client to the application server
    /// for the specified channel
    CreateTcpConnectionForChannel {
        channel: ChannelId,
        new_connection: ConnectionId,
    },

    /// Notifies the client that the DSRP server has rejected a registration request, usually
    /// due to the port being requested still being in use.
    NotifyRegistrationFailed {
        request: RequestId,
        cause: RegistrationFailureCause,
    },

    /// Notifies the client that the TCP connection was closed from the client to the DSRP server,
    /// and that the client should close the corresponding connection from the DSRP client to
    /// the application server.
    CloseTcpConnection {
        channel: ChannelId,
        connection: ConnectionId,
    },

    /// A data packet should be sent to the application server over the specified channel (or
    /// connection for a tcp channel).
    RelayRemotePacket {
        channel: ChannelId,
        connection: Option<ConnectionId>,
        data: Vec<u8>,
    },
}