use messages::{ClientMessage, ConnectionType, RequestId, ChannelId};

#[derive(Debug)]
pub enum OutstandingRequest {
    Registration{
        connection_type: ConnectionType,
        port: u16,
    }
}

#[derive(Debug)]
pub enum ClientOperation {
    NotifyChannelOpened {
        registered_by_request: RequestId,
        opened_channel: ChannelId,
    },

    SendMessageToServer {
        message: ClientMessage,
    }
}