mod data_structures;
mod errors;

pub use self::errors::{ServerMessageHandlingError, ServerMessageHandlingErrorKind};

use std::collections::{HashMap};
use std::num::Wrapping;
use handshake::HandshakeRequest;
use messages::{ClientMessage, ServerMessage, ConnectionType};
use messages::{RequestId};
use self::data_structures::{ClientOperation, OutstandingRequest};

pub struct ClientHandler {
    outstanding_requests: HashMap<RequestId, OutstandingRequest>,
    next_request_id: Wrapping<u32>,
}

impl ClientHandler {
    pub fn new() -> (Self, HandshakeRequest) {
        let handshake = HandshakeRequest::new();

        let client = ClientHandler {
            outstanding_requests: HashMap::new(),
            next_request_id: Wrapping(0),
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
            ServerMessage::RegistrationSuccessful {request, created_channel} => {
                match self.outstanding_requests.remove(&request) {
                    Some(_) => (),
                    None => {
                        let kind = ServerMessageHandlingErrorKind::UnknownRequest(request);
                        return Err(ServerMessageHandlingError {kind});
                    }
                };

                let notification = ClientOperation::NotifyChannelOpened {
                    opened_channel: created_channel,
                    registered_by_request: request,
                };

                vec![notification]
            },

            ServerMessage::DataReceived {channel: _, connection: _, data: _} => {
                unimplemented!()
            },

            ServerMessage::TcpConnectionClosed {channel: _, connection: _} => {
                unimplemented!()
            },

            ServerMessage::NewIncomingTcpConnection {channel: _, new_connection: _} => {
                unimplemented!()
            },

            ServerMessage::RegistrationFailed {request: _, cause: _} => {
                unimplemented!()
            }
        };

        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake::CURRENT_PROTOCOL_VERSION;
    use messages::ChannelId;

    #[test]
    fn new_handler_creates_handshake_request_with_current_protocol_version() {
        let (_, request) = ClientHandler::new();
        assert_eq!(request.client_protocol_version, CURRENT_PROTOCOL_VERSION, "Unexpected protocol version");
    }

    #[test]
    fn client_can_generate_tcp_port_registration_message() {
        let (mut client, _) = ClientHandler::new();
        let (request_id, message) = client.request_registration(ConnectionType::Tcp, 23);
        match message {
            ClientMessage::Register {request, connection_type, port} => {
                assert_eq!(request, request_id, "Unexpected request ID in message");
                assert_eq!(connection_type, ConnectionType::Tcp, "Unexpected connection type in message");
                assert_eq!(port, 23, "Unexpected port in message");
            },

            x => panic!("Expected Register message, instead got {:?}", x),
        }
    }

    #[test]
    fn client_can_generate_udp_port_registration_message() {
        let (mut client, _) = ClientHandler::new();
        let (request_id, message) = client.request_registration(ConnectionType::Udp, 23);
        match message {
            ClientMessage::Register {request, connection_type, port} => {
                assert_eq!(request, request_id, "Unexpected request ID in message");
                assert_eq!(connection_type, ConnectionType::Udp, "Unexpected connection type in message");
                assert_eq!(port, 23, "Unexpected port in message");
            },

            x => panic!("Expected Register message, instead got {:?}", x),
        }
    }

    #[test]
    fn can_process_valid_tcp_registration_success_result() {
        let port = 23;
        let (mut client, _) = ClientHandler::new();
        let (request_id, _) = client.request_registration(ConnectionType::Tcp, port);

        let channel = ChannelId(5);
        let response = ServerMessage::RegistrationSuccessful {
            request: request_id,
            created_channel: channel,
        };

        let results = client.handle_server_message(response).unwrap();

        assert_vec_contains!(results, ClientOperation::NotifyChannelOpened {registered_by_request, opened_channel}
        => {
            assert_eq!(*registered_by_request, request_id, "Unexpected request id returned");
            assert_eq!(*opened_channel, channel, "Unexpected channel id");
        });
    }

    #[test]
    fn can_process_valid_udp_registration_success_result() {
        let port = 23;
        let (mut client, _) = ClientHandler::new();
        let (request_id, _) = client.request_registration(ConnectionType::Udp, port);

        let channel = ChannelId(5);
        let response = ServerMessage::RegistrationSuccessful {
            request: request_id,
            created_channel: channel,
        };

        let results = client.handle_server_message(response).unwrap();

        assert_vec_contains!(results, ClientOperation::NotifyChannelOpened {registered_by_request, opened_channel}
        => {
            assert_eq!(*registered_by_request, request_id, "Unexpected request id returned");
            assert_eq!(*opened_channel, channel, "Unexpected channel id");
        });
    }

    #[test]
    fn error_if_response_does_not_match_outstanding_request_id() {
        let port = 23;
        let (mut client, _) = ClientHandler::new();
        let (request_id, _) = client.request_registration(ConnectionType::Udp, port);

        let bad_request = RequestId(request_id.0 + 1);
        let response = ServerMessage::RegistrationSuccessful {
            request: bad_request,
            created_channel: ChannelId(22),
        };

        let error = client.handle_server_message(response).unwrap_err();
        match error.kind {
            ServerMessageHandlingErrorKind::UnknownRequest(request) => {
                assert_eq!(request, bad_request, "Unexpected request in error");
            },
        }
    }
}
