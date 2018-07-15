
use std::collections::{HashSet};
use std::num::Wrapping;
use handshake::HandshakeRequest;
use messages::{ClientMessage, ConnectionType};
use messages::{RequestId};

pub struct ClientHandler {
    active_requests: HashSet<RequestId>,
    next_request_id: Wrapping<u32>,
}

impl ClientHandler {
    pub fn new() -> (Self, HandshakeRequest) {
        let handshake = HandshakeRequest::new();

        let client = ClientHandler {
            active_requests: HashSet::new(),
            next_request_id: Wrapping(0),
        };

        (client, handshake)
    }

    pub fn request_registration(&mut self, connection_type: ConnectionType, port: u16) -> (RequestId, ClientMessage) {
        let request_id;
        loop {
            self.next_request_id += Wrapping(1);
            let next_request = RequestId(self.next_request_id.0);
            if self.active_requests.contains(&next_request) {
                continue;
            }

            request_id = next_request;
            break;
        }

        self.active_requests.insert(request_id);
        let message = ClientMessage::Register {
            request: request_id,
            connection_type,
            port,
        };

        (request_id, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake::CURRENT_PROTOCOL_VERSION;

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
}
