mod data_structures;
mod errors;

pub use self::errors::{ServerMessageHandlingError, ServerMessageHandlingErrorKind};

use std::collections::{HashMap};
use std::num::Wrapping;
use handshake::HandshakeRequest;
use messages::{ClientMessage, ServerMessage, ConnectionType};
use messages::{RequestId, ChannelId};
use self::data_structures::{ClientOperation, OutstandingRequest, ActiveChannel};

pub struct ClientHandler {
    outstanding_requests: HashMap<RequestId, OutstandingRequest>,
    next_request_id: Wrapping<u32>,
    active_channels: HashMap<ChannelId, ActiveChannel>,
}

impl ClientHandler {
    pub fn new() -> (Self, HandshakeRequest) {
        let handshake = HandshakeRequest::new();

        let client = ClientHandler {
            outstanding_requests: HashMap::new(),
            next_request_id: Wrapping(0),
            active_channels: HashMap::new(),
        };

        (client, handshake)
    }

    pub fn request_registration(&mut self, connection_type: ConnectionType, port: u16) -> (RequestId, ClientMessage) {
        let request_id;
        loop {
            self.next_request_id += Wrapping(1);
            let next_request = RequestId(self.next_request_id.0);
            if self.outstanding_requests.contains_key(&next_request) {
                continue;
            }

            request_id = next_request;
            break;
        }

        let request = OutstandingRequest::Registration {connection_type: connection_type.clone(), port};
        self.outstanding_requests.insert(request_id, request);
        let message = ClientMessage::Register {
            request: request_id,
            connection_type,
            port,
        };

        (request_id, message)
    }

    pub fn handle_server_message(&mut self, message: ServerMessage) -> Result<Vec<ClientOperation>, ServerMessageHandlingError> {
        let operations = match message {
            ServerMessage::RegistrationSuccessful {request: request_id, created_channel} => {
                let request = match self.outstanding_requests.remove(&request_id) {
                    Some(x) => x,
                    None => {
                        let kind = ServerMessageHandlingErrorKind::UnknownRequest(request_id);
                        return Err(ServerMessageHandlingError {kind});
                    }
                };

                match request {
                    OutstandingRequest::Registration {connection_type, port: _} => {
                        let active_channel = ActiveChannel {connection_type};
                        self.active_channels.insert(created_channel, active_channel);

                        let notification = ClientOperation::NotifyChannelOpened {
                            opened_channel: created_channel,
                            registered_by_request: request_id,
                        };

                        vec![notification]
                    }
                }
            },

            ServerMessage::DataReceived {channel: _, connection: _, data: _} => {
                unimplemented!()
            },

            ServerMessage::TcpConnectionClosed {channel: _, connection: _} => {
                unimplemented!()
            },

            ServerMessage::NewIncomingTcpConnection {channel: channel_id, new_connection} => {
                let channel = match self.active_channels.get(&channel_id) {
                    Some(x) => x,
                    None => return Ok(Vec::new()),
                };

                if channel.connection_type != ConnectionType::Tcp {
                    return Ok(Vec::new());
                }

                let notification = ClientOperation::NotifyNewRemoteTcpConnection {
                    channel: channel_id,
                    new_connection,
                };

                vec![notification]
            },

            ServerMessage::RegistrationFailed {request: _, cause: _} => {
                unimplemented!()
            }
        };

        Ok(operations)
    }
}

#[cfg(test)]
mod tests;
