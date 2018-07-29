use super::*;
use handshake::CURRENT_PROTOCOL_VERSION;
use messages::{ChannelId, ConnectionId};

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

#[test]
fn notification_raised_when_dsrp_server_reports_new_incoming_tcp_connection() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);

    let connection1 = ConnectionId(55);
    let message = ServerMessage::NewIncomingTcpConnection {
        channel: channel1,
        new_connection: connection1,
    };

    let results = client.handle_server_message(message).unwrap();
    assert_vec_contains!(results, ClientOperation::NotifyNewRemoteTcpConnection {channel, new_connection}
    => {
        assert_eq!(*channel, channel1, "Unexpected channel identifier");
        assert_eq!(*new_connection, connection1, "Unexpected new connection identifier");
    });
}

#[test]
fn no_operation_when_dsrp_server_reports_connection_over_unknown_channel() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);

    let connection1 = ConnectionId(55);
    let message = ServerMessage::NewIncomingTcpConnection {
        channel: ChannelId(channel1.0 + 1),
        new_connection: connection1,
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

#[test]
fn no_operation_when_dsrp_server_reports_connection_over_udp_channel() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Udp, 23);

    let connection1 = ConnectionId(55);
    let message = ServerMessage::NewIncomingTcpConnection {
        channel: channel1,
        new_connection: connection1,
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

fn open_channel(client: &mut ClientHandler, connection_type: ConnectionType, port: u16) -> ChannelId {
    let (request_id, _) = client.request_registration(connection_type, port);
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

    channel
}