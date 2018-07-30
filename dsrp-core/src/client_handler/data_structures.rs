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
    /// has been accepted for the specified channel
    NotifyNewRemoteTcpConnection {
        channel: ChannelId,
        new_connection: ConnectionId,
    },

    /// Notifies the client that the DSRP server has rejected a registration request, usually
    /// due to the port being requested still being in use.
    NotifyRegistrationFailed {
        request: RequestId,
        cause: RegistrationFailureCause,
    }
}