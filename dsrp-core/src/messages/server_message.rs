use super::{RequestId, ChannelId, ConnectionId};

pub enum ServerMessage {
    /// Tells the client that their registration request was successful, and defines a
    /// channel id that will be used for communicating traffic information for the registered
    /// tcp or udp port.
    RegistrationSuccessful {
        request: RequestId,
        created_channel: ChannelId,
    },

    /// Informs the client that their registration request was not successful, and the
    /// reason why.
    RegistrationFailed {
        request: RequestId,
        cause: RegistrationFailureCause,
    },

    /// Informs the client that a new TCP connection was established on the DSRP server for
    /// a specific channel.  It establishes a connection id that will be used to communicate
    /// traffic specific to this single connection.
    NewIncomingTcpConnection {
        channel: ChannelId,
        new_connection: ConnectionId
    },

    /// Informs the client that an established TCP connection to the DSRP server was closed
    /// by the originator.
    TcpConnectionClosed {
        channel: ChannelId,
        connection: ConnectionId,
    },

    /// Data was received by the DSRP server.  If the data came over a TCP connection we provide
    /// the identifier for the connection id it was received on.
    DataReceived {
        channel: ChannelId,
        connection: Option<ConnectionId>,
        data: Vec<u8>,
    },
}

pub enum RegistrationFailureCause {
    PortAlreadyRegistered,
}