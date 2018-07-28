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
mod tests;
