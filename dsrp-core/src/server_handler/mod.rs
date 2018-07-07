use std::collections::{HashSet, HashMap};
use std::fmt;
use std::num::Wrapping;
use failure::{Backtrace, Fail};
use ::handshake::{HandshakeRequest, HandshakeResponse, CURRENT_PROTOCOL_VERSION};
use ::messages::{ClientMessage, ServerMessage, ChannelId, RegistrationFailureCause};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ClientId(pub(crate) u32);

#[derive(Debug)]
pub struct NewClient {
    id: ClientId,
    response: HandshakeResponse,
}

/// Contains the logic for handling the logic of a DSRP server
pub struct ServerHandler {
    active_clients: HashSet<ClientId>,
    active_ports: HashMap<u16, ChannelId>,
    active_channels: HashSet<ChannelId>,
    next_client_id: Wrapping<u32>,
    next_channel_id: Wrapping<u32>,
}

#[derive(Debug)]
pub struct ClientMessageHandlingError {
    pub kind: ClientMessageHandlingErrorKind,
}

#[derive(Debug, Fail)]
pub enum ClientMessageHandlingErrorKind {
    #[fail(display = "Unknown client id: {}", _0)]
    UnknownClientId(ClientId),
}

impl ServerHandler {
    pub fn new() -> Self {
        ServerHandler {
            active_clients: HashSet::new(),
            active_ports: HashMap::new(),
            active_channels: HashSet::new(),
            next_client_id: Wrapping(0),
            next_channel_id: Wrapping(0),
        }
    }

    pub fn add_client(&mut self, request: HandshakeRequest) -> Result<NewClient, HandshakeResponse> {
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
            if self.active_clients.contains(&client_id) {
                continue;
            }

            self.active_clients.insert(client_id);
            break;
        }

        let new_client = NewClient {
            id: client_id,
            response: HandshakeResponse::Success,
        };

        Ok(new_client)
    }

    pub fn remove_client(&mut self, client_id: ClientId) {
        self.active_clients.remove(&client_id);
    }

    pub fn handle_client_message(&mut self, client_id: ClientId, message: ClientMessage)
        -> Result<ServerMessage, ClientMessageHandlingError> {

        if !self.active_clients.contains(&client_id) {
            let kind = ClientMessageHandlingErrorKind::UnknownClientId(client_id);
            return Err(ClientMessageHandlingError {kind});
        }

        let response = match message {
            ClientMessage::Register {request, connection_type: _, port} => {
                if self.active_ports.contains_key(&port) {
                    ServerMessage::RegistrationFailed {
                        request,
                        cause: RegistrationFailureCause::PortAlreadyRegistered,
                    }
                }
                else {
                    let mut channel_id;
                    loop {
                        self.next_channel_id = Wrapping(self.next_channel_id.0 + 1);
                        channel_id = ChannelId(self.next_channel_id.0);
                        if self.active_channels.contains(&channel_id) {
                            continue;
                        }

                        break;
                    }

                    self.active_ports.insert(port, channel_id);
                    self.active_channels.insert(channel_id);
                    ServerMessage::RegistrationSuccessful {
                        request,
                        created_channel: channel_id,
                    }
                }
            },

            x => panic!("Unsupported client message: {:?}", x),
        };

        Ok(response)
    }
}

impl fmt::Display for ClientMessageHandlingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl Fail for ClientMessageHandlingError {
    fn cause(&self) -> Option<&Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::messages::{ConnectionType, RequestId};

    #[test]
    fn can_create_client_with_current_handshake_protocol_version() {
        let handshake = HandshakeRequest::new();
        let mut handler = ServerHandler::new();
        let new_client = handler.add_client(handshake).unwrap();

        assert_eq!(new_client.response, HandshakeResponse::Success, "Unexpected handshake response");
    }

    #[test]
    fn cannot_create_client_with_incorrect_handshake_protocol_version() {
        let handshake = HandshakeRequest {client_protocol_version: CURRENT_PROTOCOL_VERSION + 1};
        let mut handler = ServerHandler::new();
        let error = handler.add_client(handshake).unwrap_err();

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
        let new_client1 = handler.add_client(handshake1).unwrap();
        let new_client2 = handler.add_client(handshake2).unwrap();

        assert_ne!(new_client1.id, new_client2.id, "Expected clients to have different ids")
    }

    #[test]
    fn error_when_handling_message_from_unknown_client_id() {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_client(HandshakeRequest::new()).unwrap();

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

            //x => panic!("Expected unknown client id error, instead received: {}", x),
        }
    }

    #[test]
    fn client_can_register_unused_tcp_port() {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        match response {
            ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
            },

            x => panic!("Expected registration successful message, instead got: {:?}", x),
        }
    }

    #[test]
    fn client_cannot_register_single_port_twice_for_same_protocol()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        match response {
            ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
            },

            x => panic!("Expected registration successful message, instead got: {:?}", x),
        }

        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        match response {
            ServerMessage::RegistrationFailed {request: response_request_id, cause} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
                assert_eq!(cause, RegistrationFailureCause::PortAlreadyRegistered, "Unexpected cause");
            },

            x => panic!("Expected registration failure message, instead got: {:?}", x),
        }
    }

    #[test]
    fn client_cannot_register_single_port_twice_for_different_protocols()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        match response {
            ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
            },

            x => panic!("Expected registration successful message, instead got: {:?}", x),
        }

        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Udp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id, message).unwrap();
        match response {
            ServerMessage::RegistrationFailed {request: response_request_id, cause} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
                assert_eq!(cause, RegistrationFailureCause::PortAlreadyRegistered, "Unexpected cause");
            },

            x => panic!("Expected registration failure message, instead got: {:?}", x),
        }
    }

    #[test]
    fn different_clients_cannot_request_same_port() {
        let mut handler = ServerHandler::new();
        let new_client1 = handler.add_client(HandshakeRequest::new()).unwrap();
        let new_client2 = handler.add_client(HandshakeRequest::new()).unwrap();

        let client_id1 = new_client1.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id1, message).unwrap();
        match response {
            ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel: _} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
            },

            x => panic!("Expected registration successful message, instead got: {:?}", x),
        }

        let client_id2 = new_client2.id;
        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Udp,
            port: 23,
            request: request_id,
        };

        let response = handler.handle_client_message(client_id2, message).unwrap();
        match response {
            ServerMessage::RegistrationFailed {request: response_request_id, cause} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
                assert_eq!(cause, RegistrationFailureCause::PortAlreadyRegistered, "Unexpected cause");
            },

            x => panic!("Expected registration failure message, instead got: {:?}", x),
        }
    }

    #[test]
    fn multiple_registrations_return_different_channel_ids()
    {
        let mut handler = ServerHandler::new();
        let new_client = handler.add_client(HandshakeRequest::new()).unwrap();

        let client_id = new_client.id;
        let request_id = RequestId(25);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 23,
            request: request_id,
        };

        let channel1;
        let response = handler.handle_client_message(client_id, message).unwrap();
        match response {
            ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
                channel1 = created_channel;
            },

            x => panic!("Expected registration successful message, instead got: {:?}", x),
        }

        let request_id = RequestId(26);
        let message = ClientMessage::Register {
            connection_type: ConnectionType::Tcp,
            port: 24,
            request: request_id,
        };

        let channel2;
        let response = handler.handle_client_message(client_id, message).unwrap();
        match response {
            ServerMessage::RegistrationSuccessful {request: response_request_id, created_channel} => {
                assert_eq!(response_request_id, request_id, "Unexpected request id in response");
                channel2 = created_channel;
            },

            x => panic!("Expected registration successful message, instead got: {:?}", x),
        }

        assert_ne!(channel1, channel2, "Both channels were not supposed to be the same ids");
    }
}