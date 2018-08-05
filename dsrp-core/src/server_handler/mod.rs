mod errors;
mod data_structures;

use std::collections::{HashSet, HashMap};
use std::num::Wrapping;
use ::handshake::{HandshakeRequest, HandshakeResponse, CURRENT_VERSION};
use ::messages::{ClientMessage, ServerMessage, ChannelId, RegistrationFailureCause};
use ::messages::{ConnectionType, ConnectionId};
use self::data_structures::{ActiveChannel, ActiveClient};

pub use self::errors::{ClientMessageHandlingError, ClientMessageHandlingErrorKind};
pub use self::errors::{NewConnectionError, NewConnectionErrorKind};
pub use self::data_structures::{NewClient, ClientId, ServerOperation, ActiveTcpConnection};

/// Contains the logic for handling the logic of a DSRP server
pub struct ServerHandler {
    active_clients: HashMap<ClientId, ActiveClient>,
    active_ports: HashMap<u16, ChannelId>,
    active_channels: HashMap<ChannelId, ActiveChannel>,
    active_tcp_connections: HashMap<ConnectionId, ActiveTcpConnection>,
    next_client_id: Wrapping<u32>,
    next_channel_id: Wrapping<u32>,
    next_connection_id: Wrapping<u32>,
}

impl ServerHandler {
    pub fn new() -> Self {
        ServerHandler {
            active_clients: HashMap::new(),
            active_ports: HashMap::new(),
            active_channels: HashMap::new(),
            active_tcp_connections: HashMap::new(),
            next_client_id: Wrapping(0),
            next_channel_id: Wrapping(0),
            next_connection_id: Wrapping(0),
        }
    }

    pub fn add_dsrp_client(&mut self, request: HandshakeRequest) -> Result<NewClient, HandshakeResponse> {
        // For now only accept clients running the same version as the server
        if request.client_protocol_version != CURRENT_VERSION {
            let message = format!("Protocol version {} requested but only protocol version {} is supported",
                request.client_protocol_version,
                                  CURRENT_VERSION);
            return Err(HandshakeResponse::Failure {reason: message});
        }

        let mut client_id;
        loop {
            self.next_client_id = self.next_client_id + Wrapping(1);
            client_id = ClientId(self.next_client_id.0);
            if self.active_clients.contains_key(&client_id) {
                continue;
            }

            let client = ActiveClient { channels: HashSet::new() };
            self.active_clients.insert(client_id, client);
            break;
        }

        let new_client = NewClient {
            id: client_id,
            response: HandshakeResponse::Success,
        };

        Ok(new_client)
    }

    pub fn remove_dsrp_client(&mut self, client_id: ClientId) -> Vec<ServerOperation> {
        let mut results = Vec::new();
        let client = match self.active_clients.remove(&client_id) {
            Some(x) => x,
            None => return Vec::new(),
        };

        for channel in client.channels {
            let active_channel = match self.active_channels.remove(&channel) {
                Some(x) => x,
                None => continue,
            };

            self.active_ports.remove(&active_channel.port);
            let operation = match active_channel.connection_type {
                ConnectionType::Tcp => ServerOperation::StopTcpOperations {port: active_channel.port},
                ConnectionType::Udp => ServerOperation::StopUdpOperations {port: active_channel.port},
            };

            results.push(operation);

            for connection in active_channel.tcp_connections {
                self.active_tcp_connections.remove(&connection);
                results.push(ServerOperation::DisconnectConnection {connection});
            }
        }

        results
    }

    pub fn handle_client_message(&mut self, client_id: ClientId, message: ClientMessage)
        -> Result<Vec<ServerOperation>, ClientMessageHandlingError> {

        if !self.active_clients.contains_key(&client_id) {
            let kind = ClientMessageHandlingErrorKind::UnknownClientId(client_id);
            return Err(ClientMessageHandlingError {kind});
        }

        let response = match message {
            ClientMessage::Register {request, connection_type, port} => {
                if self.active_ports.contains_key(&port) {
                    vec![ServerOperation::SendMessageToDsrpClient {
                        client: client_id,
                        message: ServerMessage::RegistrationFailed {
                            request,
                            cause: RegistrationFailureCause::PortAlreadyRegistered,
                        }
                    }]
                }
                else {
                    let mut channel_id;
                    loop {
                        self.next_channel_id = self.next_channel_id + Wrapping(1);
                        channel_id = ChannelId(self.next_channel_id.0);
                        if self.active_channels.contains_key(&channel_id) {
                            continue;
                        }

                        break;
                    }

                    let channel = ActiveChannel {
                        port,
                        connection_type: connection_type.clone(),
                        owner: client_id,
                        tcp_connections: HashSet::new(),
                    };

                    self.active_ports.insert(port, channel_id);
                    self.active_channels.insert(channel_id, channel);

                    // Unwrap should be safe here due to if statement above verifying the client exists
                    let client = self.active_clients.get_mut(&client_id).unwrap();
                    client.channels.insert(channel_id);

                    let message_to_client = ServerOperation::SendMessageToDsrpClient {
                        client: client_id,
                        message: ServerMessage::RegistrationSuccessful {
                            request,
                            created_channel: channel_id,
                        }
                    };

                    let start_operation = match connection_type {
                        ConnectionType::Tcp => ServerOperation::StartTcpOperations {port, channel: channel_id},
                        ConnectionType::Udp => ServerOperation::StartUdpOperations {port, channel: channel_id},
                    };

                    vec![start_operation, message_to_client]
                }
            },

            ClientMessage::Unregister {channel} => {
                let port;
                let connection_type;
                let connection_ids;
                {
                    let channel_details = self.active_channels.get(&channel);
                    if let None = channel_details {
                        let kind = ClientMessageHandlingErrorKind::ChannelNotFound(channel);
                        return Err(ClientMessageHandlingError { kind });
                    }

                    let channel_details = channel_details.unwrap();
                    if channel_details.owner != client_id {
                        let kind = ClientMessageHandlingErrorKind::ChannelNotOwnedByRequester {
                            channel,
                            requesting_client: client_id,
                            owning_client: channel_details.owner,
                        };

                        return Err(ClientMessageHandlingError { kind });
                    }

                    port = channel_details.port;
                    connection_type = channel_details.connection_type.clone();
                    connection_ids = channel_details.tcp_connections.clone();
                }

                let mut operations = Vec::new();
                for connection in connection_ids {
                    self.active_tcp_connections.remove(&connection);
                    operations.push(ServerOperation::DisconnectConnection {connection });
                }

                let operation = match connection_type {
                    ConnectionType::Tcp => ServerOperation::StopTcpOperations {port},
                    ConnectionType::Udp => ServerOperation::StopUdpOperations {port},
                };

                self.active_ports.remove(&port);
                self.active_channels.remove(&channel);

                operations.push(operation);
                operations
            },

            ClientMessage::TcpConnectionDisconnected {channel: channel_id, connection: connection_id} => {
                self.handle_dsrp_client_disconnection_notification(client_id, channel_id, connection_id)
            }

            ClientMessage::DataBeingSent {channel: channel_id, connection: connection_id, data} => {
                self.handle_dsrp_client_data_sent_message(client_id, channel_id, connection_id, data)
            },
        };

        Ok(response)
    }

    pub fn new_channel_tcp_connection(&mut self, channel_id: ChannelId)
        -> Result<(ConnectionId, ServerOperation), NewConnectionError> {
        let channel = match self.active_channels.get_mut(&channel_id) {
            Some(x) => x,
            None => {
                let kind = NewConnectionErrorKind::UnknownChannelId(channel_id);
                return Err(NewConnectionError {kind});
            },
        };

        match channel.connection_type {
            ConnectionType::Tcp => (),
            _ => {
                let kind = NewConnectionErrorKind::ConnectionAddedToNonTcpChannel(channel_id);
                return Err(NewConnectionError {kind});
            }
        }

        let mut new_connection_id;
        loop {
            self.next_connection_id = self.next_connection_id + Wrapping(1);
            new_connection_id = ConnectionId(self.next_connection_id.0);
            if self.active_tcp_connections.contains_key(&new_connection_id) {
                continue;
            }

            let connection = ActiveTcpConnection {
                owning_channel: channel_id,
                owning_client: channel.owner,
            };

            self.active_tcp_connections.insert(new_connection_id, connection);
            channel.tcp_connections.insert(new_connection_id);
            break;
        }

        let operation = ServerOperation::SendMessageToDsrpClient {
            client: channel.owner,
            message: ServerMessage::NewIncomingTcpConnection {
                new_connection: new_connection_id,
                channel: channel_id
            }
        };

        Ok((new_connection_id, operation))
    }

    pub fn tcp_connection_disconnected(&mut self, connection_id: ConnectionId) -> Option<ServerOperation> {
        let connection = match self.active_tcp_connections.remove(&connection_id) {
            Some(x) => x,
            None => return None,
        };

        let channel = self.active_channels.get_mut(&connection.owning_channel);

        if let Some(x) = channel {
            x.tcp_connections.remove(&connection_id);
            Some(ServerOperation::SendMessageToDsrpClient {
                client: x.owner,
                message: ServerMessage::TcpConnectionClosed {
                    channel: connection.owning_channel,
                    connection: connection_id,
                }
            })
        } else {
            None
        }
    }

    pub fn tcp_data_received(&self, connection_id: ConnectionId, data: &[u8]) -> Option<ServerOperation> {
        let connection = match self.active_tcp_connections.get(&connection_id) {
            Some(x) => x,
            None => return None,
        };

        let mut data_copy = Vec::new();
        data_copy.extend_from_slice(data);

        let message = ServerMessage::DataReceived {
            channel: connection.owning_channel,
            connection: Some(connection_id),
            data: data_copy,
        };

        let operation = ServerOperation::SendMessageToDsrpClient {
            client: connection.owning_client,
            message
        };
        Some(operation)
    }

    pub fn udp_data_received(&self, channel_id: ChannelId, data: &[u8]) -> Option<ServerOperation> {
        let channel = match self.active_channels.get(&channel_id) {
            Some(x) => x,
            None => return None,
        };

        let mut data_copy = Vec::new();
        data_copy.extend_from_slice(data);

        let message = ServerMessage::DataReceived {
            channel: channel_id,
            connection: None,
            data: data_copy,
        };

        let operation = ServerOperation::SendMessageToDsrpClient {
            client: channel.owner,
            message
        };

        Some(operation)
    }

    fn handle_dsrp_client_disconnection_notification(&mut self,
                                                     client_id: ClientId,
                                                     channel_id: ChannelId,
                                                     connection_id: ConnectionId)
        -> Vec<ServerOperation> {

        // Validations
        let channel;
        {
            let connection = match self.active_tcp_connections.get(&connection_id) {
                Some(x) => x,
                None => return Vec::new(),
            };

            channel = match self.active_channels.get_mut(&channel_id) {
                Some(x) => x,
                None => return Vec::new(),
            };

            if connection.owning_channel != channel_id || channel.owner != client_id {
                return Vec::new()
            }
        }

        // If we got here without an early return then all validations check out
        self.active_tcp_connections.remove(&connection_id);
        channel.tcp_connections.remove(&connection_id);

        vec![ServerOperation::DisconnectConnection {connection: connection_id}]
    }

    fn handle_dsrp_client_data_sent_message(&self,
                                            client_id: ClientId,
                                            channel_id: ChannelId,
                                            connection_id: Option<ConnectionId>,
                                            data: Vec<u8>) -> Vec<ServerOperation> {
        let channel = match self.active_channels.get(&channel_id) {
            Some(x) => x,
            None => return Vec::new(), // channel doesn't exist
        };

        if channel.owner != client_id {
            return Vec::new(); // channel is not owned by this client
        }

        if channel.connection_type == ConnectionType::Tcp {
            if let Some(id) = connection_id {
                if !channel.tcp_connections.contains(&id) {
                    return Vec::new(); // Not an active connection for this channel
                }
            } else {
                return Vec::new(); // tcp channels must have a connection
            }
        } else {
            if connection_id.is_some() {
                return Vec::new(); // UDP channels do not use connections
            }
        }

        // If we got here that means this is a valid request to relay
        vec![ServerOperation::SendByteData {
            channel: channel_id,
            connection: connection_id,
            data,
        }]
    }
}

#[cfg(test)]
mod tests;