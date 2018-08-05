mod data_structures;
mod errors;

pub use self::errors::{ServerMessageHandlingError, ServerMessageHandlingErrorKind};

use std::collections::{HashMap, HashSet};
use std::num::Wrapping;
use handshake::HandshakeRequest;
use messages::{ClientMessage, ServerMessage, ConnectionType};
use messages::{RequestId, ChannelId, ConnectionId};
use self::data_structures::{ClientOperation, OutstandingRequest, ActiveChannel, ActiveConnection};

pub struct ClientHandler {
    outstanding_requests: HashMap<RequestId, OutstandingRequest>,
    next_request_id: Wrapping<u32>,
    active_channels: HashMap<ChannelId, ActiveChannel>,
    active_connections: HashMap<ConnectionId, ActiveConnection>,
}

impl ClientHandler {
    pub fn new() -> (Self, HandshakeRequest) {
        let handshake = HandshakeRequest::new();

        let client = ClientHandler {
            outstanding_requests: HashMap::new(),
            next_request_id: Wrapping(0),
            active_channels: HashMap::new(),
            active_connections: HashMap::new(),
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
                        let active_channel = ActiveChannel {
                            connection_type,
                            connections: HashSet::new(),
                        };

                        self.active_channels.insert(created_channel, active_channel);

                        let notification = ClientOperation::NotifyChannelOpened {
                            opened_channel: created_channel,
                            registered_by_request: request_id,
                        };

                        vec![notification]
                    }
                }
            },

            ServerMessage::DataReceived {channel: channel_id, connection: connection_id, data} => {
                let channel = match self.active_channels.get(&channel_id) {
                    Some(x) => x,
                    None => return Ok(Vec::new()),
                };

                match channel.connection_type {
                    ConnectionType::Tcp => {
                        if connection_id.is_none() || !channel.connections.contains(&connection_id.unwrap()) {
                            return Ok(Vec::new()); // all tcp messages should be over a specific connection
                        }
                    },

                    ConnectionType::Udp => {
                        if connection_id.is_some() {
                            return Ok(Vec::new()); // A specific connection is not valid for udp channels
                        }
                    },
                }

                vec![ClientOperation::RelayRemotePacket {
                    channel: channel_id,
                    connection: connection_id,
                    data,
                }]
            },

            ServerMessage::TcpConnectionClosed {channel: channel_id, connection: connection_id} => {
                {
                    // Validations
                    let connection = match self.active_connections.get(&connection_id) {
                        Some(x) => x,
                        None => return Ok(Vec::new()),
                    };

                    if connection.owner != channel_id {
                        return Ok(Vec::new());
                    }
                }

                // Validations passed
                let channel = match self.active_channels.get_mut(&channel_id) {
                    Some(x) => x,
                    None => return Ok(Vec::new()),
                };

                self.active_connections.remove(&connection_id);
                channel.connections.remove(&connection_id);

                let operation = ClientOperation::CloseTcpConnection {
                    channel: channel_id,
                    connection: connection_id,
                };

                vec![operation]

            },

            ServerMessage::NewIncomingTcpConnection {channel: channel_id, new_connection} => {
                let channel = match self.active_channels.get_mut(&channel_id) {
                    Some(x) => x,
                    None => return Ok(Vec::new()),
                };

                if channel.connection_type != ConnectionType::Tcp {
                    return Ok(Vec::new());
                }

                let active_connection = ActiveConnection {owner: channel_id};
                channel.connections.insert(new_connection);
                self.active_connections.insert(new_connection, active_connection);

                let operation = ClientOperation::CreateTcpConnectionForChannel {
                    channel: channel_id,
                    new_connection,
                };

                vec![operation]
            },

            ServerMessage::RegistrationFailed {request: request_id, cause} => {
                match self.outstanding_requests.remove(&request_id) {
                    Some(_) => (),
                    None => {
                        let kind = ServerMessageHandlingErrorKind::UnknownRequest(request_id);
                        return Err(ServerMessageHandlingError {kind});
                    },
                }

                vec![ClientOperation::NotifyRegistrationFailed {
                    request: request_id,
                    cause,
                }]
            }
        };

        Ok(operations)
    }
}

#[cfg(test)]
mod tests;
