mod errors;
mod data_structures;

use std::collections::{HashSet, HashMap};
use std::num::Wrapping;
use ::handshake::{HandshakeRequest, HandshakeResponse, CURRENT_PROTOCOL_VERSION};
use ::messages::{ClientMessage, ServerMessage, ChannelId, RegistrationFailureCause};
use ::messages::{ConnectionType};
use self::data_structures::{ActiveChannel, ActiveClient};

pub use self::errors::{ClientMessageHandlingError, ClientMessageHandlingErrorKind};
pub use self::data_structures::{NewClient, ClientId, ServerOperation};

/// Contains the logic for handling the logic of a DSRP server
pub struct ServerHandler {
    active_clients: HashMap<ClientId, ActiveClient>,
    active_ports: HashMap<u16, ChannelId>,
    active_channels: HashMap<ChannelId, ActiveChannel>,
    next_client_id: Wrapping<u32>,
    next_channel_id: Wrapping<u32>,
}

impl ServerHandler {
    pub fn new() -> Self {
        ServerHandler {
            active_clients: HashMap::new(),
            active_ports: HashMap::new(),
            active_channels: HashMap::new(),
            next_client_id: Wrapping(0),
            next_channel_id: Wrapping(0),
        }
    }

    pub fn add_dsrp_client(&mut self, request: HandshakeRequest) -> Result<NewClient, HandshakeResponse> {
        // For now only accept clients running the same version as the server
        if request.client_protocol_version != CURRENT_PROTOCOL_VERSION {
            let message = format!("Protocol version {} requested but only protocol version {} is supported",
                request.client_protocol_version,
                CURRENT_PROTOCOL_VERSION);
            return Err(HandshakeResponse::Failure {reason: message});
        }

        let mut client_id;
        loop {
            self.next_client_id = Wrapping(self.next_client_id.0 + 1);
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
        match self.active_clients.remove(&client_id) {
            None => (),
            Some(client) => {
                for channel in client.channels {
                    match self.active_channels.remove(&channel) {
                        None => (),
                        Some(active_channel) => {
                            self.active_ports.remove(&active_channel.port);
                            let operation = match active_channel.connection_type {
                                ConnectionType::Tcp => ServerOperation::StopTcpOperations {
                                    port: active_channel.port,
                                },

                                ConnectionType::Udp => ServerOperation::StopUdpOperations {
                                    port: active_channel.port,
                                },
                            };

                            results.push(operation);
                        }
                    }
                }
            }
        }

        results
    }

    pub fn handle_client_message(&mut self, client_id: ClientId, message: ClientMessage)
        -> Result<Vec<ServerOperation>, ClientMessageHandlingError> {

        let client = match self.active_clients.get_mut(&client_id) {
            Some(x) => x,
            None => {
                let kind = ClientMessageHandlingErrorKind::UnknownClientId(client_id);
                return Err(ClientMessageHandlingError {kind});
            }
        };

        let response = match message {
            ClientMessage::Register {request, connection_type, port} => {
                if self.active_ports.contains_key(&port) {
                    vec![ServerOperation::SendMessageToDsrpClient {
                        message: ServerMessage::RegistrationFailed {
                            request,
                            cause: RegistrationFailureCause::PortAlreadyRegistered,
                        }
                    }]
                }
                else {
                    let mut channel_id;
                    loop {
                        self.next_channel_id = Wrapping(self.next_channel_id.0 + 1);
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
                    };

                    self.active_ports.insert(port, channel_id);
                    self.active_channels.insert(channel_id, channel);
                    client.channels.insert(channel_id);

                    let message_to_client = ServerOperation::SendMessageToDsrpClient {
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
                }

                self.active_ports.remove(&port);
                self.active_channels.remove(&channel);

                let operation = match connection_type {
                    ConnectionType::Tcp => ServerOperation::StopTcpOperations {port},
                    ConnectionType::Udp => ServerOperation::StopUdpOperations {port},
                };

                vec![operation]
            },

            x => panic!("Unsupported client message: {:?}", x),
        };

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::u32;
    use ::messages::{ConnectionType, RequestId};

    #[test]
    fn can_create_client_with_current_handshake_protocol_version() {
        let handshake = HandshakeRequest::new();
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(handshake).unwrap();

        assert_eq!(new_client.response, HandshakeResponse::Success, "Unexpected handshake response");
    }

    #[test]
    fn cannot_create_client_with_incorrect_handshake_protocol_version() {
        let handshake = HandshakeRequest {client_protocol_version: CURRENT_PROTOCOL_VERSION + 1};
        let mut handler = ServerHandler::new();
        let error = handler.add_dsrp_client(handshake).unwrap_err();

        match error {
            HandshakeResponse::Failure {reason: _} => (),
            x => panic!("Expected failure, instead got {:?}", x),
        }
    }

    #[test]
    fn multiple_clients_have_different_ids()
    {
        let handshake1 = HandshakeRequest::new();
        let handshake2 = HandshakeRequest::new();
        let mut handler = ServerHandler::new();
        let new_client1 = handler.add_dsrp_client(handshake1).unwrap();
        let new_client2 = handler.add_dsrp_client(handshake2).unwrap();

        assert_ne!(new_client1.id, new_client2.id, "Expected clients to have different ids")
    }

    #[test]
    fn error_when_handling_message_from_unknown_client_id() {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = ClientId(new_client.id.0 + 1);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: RequestId(25),
        };

        let error = handler.handle_client_message(client_id, message).unwrap_err();
        match error.kind {
            ClientMessageHandlingErrorKind::UnknownClientId(err_client_id) => {
                assert_eq!(err_client_id, client_id, "Unexepcted client id in error");
            },

            x => panic!("Expected unknown client id error, instead received: {}", x),
        }
    }

    #[test]
    fn client_can_register_unused_tcp_port() {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();

        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
                message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _}
        } => {
                    assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
        });
    }

    #[test]
    fn client_cannot_register_single_port_twice_for_same_protocol()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
        });

        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationFailed {request: response_request_id, cause}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
                assert_eq!(*cause, RegistrationFailureCause::PortAlreadyRegistered, "Unexpected cause");
        });
    }

    #[test]
    fn client_cannot_register_single_port_twice_for_different_protocols()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
        });

        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Udp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationFailed {request: response_request_id, cause}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
                assert_eq!(*cause, RegistrationFailureCause::PortAlreadyRegistered, "Unexpected cause");
        });
    }

    #[test]
    fn different_clients_cannot_request_same_port() {
        let mut handler = ServerHandler::new();
        let new_client1 = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();
        let new_client2 = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id1 = new_client1.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id1, message).unwrap();
        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
        });

        let client_id2 = new_client2.id;
        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Udp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id2, message).unwrap();
        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationFailed {request: response_request_id, cause}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
                assert_eq!(*cause, RegistrationFailureCause::PortAlreadyRegistered, "Unexpected cause");
        });
    }

    #[test]
    fn multiple_registrations_return_different_channel_ids()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let mut channel1 = ChannelId(u32::MAX);
        let response = handler.handle_client_message(client_id, message).unwrap();
        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
                channel1 = *created_channel;
        });

        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 24,
            request: request_id,
        };

        let mut channel2 = ChannelId(u32::MAX);
        let response2 = handler.handle_client_message(client_id, message).unwrap();
        assert_vec_contains!(response2, ServerOperation::SendMessageToDsrpClient {
            message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
                channel2 = *created_channel;
        });

        assert_ne!(channel1, channel2, "Both channels were not supposed to be the same ids");
    }

    #[test]
    fn successful_tcp_registration_instructs_server_to_start_tcp_operations_with_same_channel_as_success_message()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let requested_port = 23;
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: requested_port,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();

        let mut operation_channel = ChannelId(u32::MAX);
        let mut success_channel = ChannelId(u32::MAX);

        assert_vec_contains!(response, ServerOperation::StartTcpOperations {port, channel}
            => {
            assert_eq!(*port, requested_port, "Incorrect port in operation");
            operation_channel = *channel
        });

        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
                message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
                success_channel = *created_channel;
        });

        assert_eq!(operation_channel, success_channel, "Non-matching channel ids");
        assert_ne!(operation_channel, ChannelId(u32::MAX), "Incorrect defined channel");
    }

    #[test]
    fn successful_udp_registration_instructs_server_to_start_udp_operations_with_same_channel_as_success_message()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let requested_port = 23;
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Udp,
            port: requested_port,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();

        let mut operation_channel = ChannelId(u32::MAX);
        let mut success_channel = ChannelId(u32::MAX);

        assert_vec_contains!(response, ServerOperation::StartUdpOperations {port, channel}
            => {
            assert_eq!(*port, requested_port, "Incorrect port in operation");
            operation_channel = *channel
        });

        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
                message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel}
        } => {
                assert_eq!(*response_request_id, request_id, "Unexpected request id in response");
                success_channel = *created_channel;
        });

        assert_eq!(operation_channel, success_channel, "Non-matching channel ids");
        assert_ne!(operation_channel, ChannelId(u32::MAX), "Incorrect defined channel");
    }

    #[test]
    fn removing_client_returns_stop_operations_on_all_registered_channels() {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request1 = RequestId(25);
        let request2 = RequestId(26);
        let port1 = 23;
        let port2 = 25;

        let message1 = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: port1,
            request: request1,
        };

        let message2 = ClientMessage::Register {
            connection_type: ConnectionType::Udp,
            port: port2,
            request: request2,
        };

        // Assume they are successful based on tests above
        let _ = handler.handle_client_message(client_id, message1).unwrap();
        let _ = handler.handle_client_message(client_id, message2).unwrap();

        let operations = handler.remove_dsrp_client(client_id);

        assert_vec_contains!(operations, ServerOperation::StopTcpOperations {port}
        => {
            assert_eq!(*port, port1, "Unexpected port for stop tcp operation");
        });

        assert_vec_contains!(operations, ServerOperation::StopUdpOperations {port}
        => {
            assert_eq!(*port, port2, "Unexpected port for stop udp operation");
        });
    }

    #[test]
    fn removing_client_reopens_port() {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request1 = RequestId(25);
        let port1 = 23;

        let message1 = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: port1,
            request: request1,
        };

        // Assume they are successful based on tests above
        let _ = handler.handle_client_message(client_id, message1).unwrap();
        let _ =  handler.remove_dsrp_client(client_id);

        // Try with new client
        let request2 = RequestId(26);
        let message1 = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: port1,
            request: request2,
        };

        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();
        let client_id = new_client.id;
        
        let response = handler.handle_client_message(client_id, message1).unwrap();

        assert_vec_contains!(response, ServerOperation::SendMessageToDsrpClient {
                message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _}
        } => {
                    assert_eq!(*response_request_id, request2, "Unexpected request id in response");
        });
    }

    #[test]
    fn unregister_request_returns_stop_operation_on_tcp_channel()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request1 = RequestId(25);
        let opened_port = 23;

        let register_message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: opened_port,
            request: request1,
        };

        let register_response = handler.handle_client_message(client_id, register_message).unwrap();
        let mut opened_channel = ChannelId(u32::MAX);
        assert_vec_contains!(register_response, ServerOperation::StartTcpOperations {port: _, channel}
        => {
            opened_channel = *channel
        });

        let unregister_message = ClientMessage::Unregister {channel: opened_channel};
        let unregister_response = handler.handle_client_message(client_id, unregister_message).unwrap();

        assert_vec_contains!(unregister_response, ServerOperation::StopTcpOperations {port}
        => {
            assert_eq!(*port, opened_port, "Unexpected port stopped");
        });
    }

    #[test]
    fn unregister_request_returns_stop_operation_on_udp_channel()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request1 = RequestId(25);
        let opened_port = 23;

        let register_message = ClientMessage::Register {
            connection_type: ConnectionType::Udp,
            port: opened_port,
            request: request1,
        };

        let register_response = handler.handle_client_message(client_id, register_message).unwrap();
        let mut opened_channel = ChannelId(u32::MAX);
        assert_vec_contains!(register_response, ServerOperation::StartUdpOperations {port: _, channel}
        => {
            opened_channel = *channel
        });

        let unregister_message = ClientMessage::Unregister {
            channel: opened_channel,
        };

        let unregister_response = handler.handle_client_message(client_id, unregister_message).unwrap();

        assert_vec_contains!(unregister_response, ServerOperation::StopUdpOperations {port}
        => {
            assert_eq!(*port, opened_port, "Unexpected port stopped");
        });
    }

    #[test]
    fn error_when_attempting_to_unregister_nonexistent_channel()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;

        let unregister_message = ClientMessage::Unregister {channel: ChannelId(22)};
        let error = handler.handle_client_message(client_id, unregister_message).unwrap_err();
        match error.kind {
            ClientMessageHandlingErrorKind::ChannelNotFound(channel_id) => {
                assert_eq!(channel_id, ChannelId(22), "Unexpected channel id in error");
            },

            x => panic!("Expected channel not found error, instead received: {}", x),
        }
    }

    #[test]
    fn error_when_attempting_to_unregister_channel_owned_by_another_client ()
    {
        let mut handler = ServerHandler::new();
        let client1 = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let request1 = RequestId(25);
        let opened_port = 23;

        let register_message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: opened_port,
            request: request1,
        };

        let register_response = handler.handle_client_message(client1.id, register_message).unwrap();
        let mut opened_channel = ChannelId(u32::MAX);
        assert_vec_contains!(register_response, ServerOperation::StartTcpOperations {port: _, channel}
        => {
            opened_channel = *channel
        });

        let client2 = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();
        let unregister_message = ClientMessage::Unregister {channel: opened_channel};
        let error = handler.handle_client_message(client2.id, unregister_message).unwrap_err();
        match error.kind {
            ClientMessageHandlingErrorKind::ChannelNotOwnedByRequester {channel, requesting_client, owning_client} => {
                assert_eq!(channel, opened_channel, "Unexpected channel id in error");
                assert_eq!(requesting_client, client2.id, "Unexpected requesting client");
                assert_eq!(owning_client, client1.id, "Unexpected owning client");
            },

            x => panic!("Expected channel not owned by requester error, instead received: {}", x),
        }
    }

    #[test]
    fn unregistering_allows_port_to_be_reused() {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_dsrp_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request1 = RequestId(25);
        let request2 = RequestId(26);
        let opened_port = 23;

        let register_message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: opened_port,
            request: request1,
        };

        let register_response = handler.handle_client_message(client_id, register_message).unwrap();

        let mut opened_channel = ChannelId(u32::MAX);
        assert_vec_contains!(register_response, ServerOperation::StartTcpOperations {port: _, channel}
        => {
            opened_channel = *channel
        });

        let unregister_message = ClientMessage::Unregister {channel: opened_channel};
        let _ = handler.handle_client_message(client_id, unregister_message).unwrap();

        let register_message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: opened_port,
            request: request2,
        };

        let register2_response = handler.handle_client_message(client_id, register_message).unwrap();
        assert_vec_contains!(register2_response, ServerOperation::SendMessageToDsrpClient {
                message: ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _}
        } => {
                    assert_eq!(*response_request_id, request2, "Unexpected request id in response");
        });
    }
}