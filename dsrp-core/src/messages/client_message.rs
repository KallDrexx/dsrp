use super::{ConnectionType, RequestId, ChannelId, ConnectionId};

pub enum ClientMessage {
    /// A Request to have the DSRP server relay all tcp or udp traffic from the specified port
    /// to the client sending the request.
    Register {
        request_id: RequestId,
        connection_type: ConnectionType,
        port: u16,
    },

    /// Tells the DSRP server that the channel should be closed, meaning the server should not
    /// relay traffic from this ch
    Unregister {
        channel: ChannelId,
    },

    /// Tells the DSRP server that the TCP connection was dropped on our end
    TcpConnectionDisconnected {
        channel: ChannelId,
        connection: ConnectionId,
    },

    /// Relays an outbound packet, so the DSRP server can relay it to the originator of the
    /// the connection.
    DataBeingSent {
        channel: ChannelId,
        connection: Option<ConnectionId>,
        data: Vec<u8>,
    },
}