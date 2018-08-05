use super::*;
use handshake::CURRENT_VERSION;
use messages::{ChannelId, ConnectionId, RegistrationFailureCause};
use rand;

#[test]
fn new_handler_creates_handshake_request_with_current_protocol_version() {
    let (_, request) = ClientHandler::new();
    assert_eq!(request.client_protocol_version, CURRENT_VERSION, "Unexpected protocol version");
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
fn registration_failed_notification_raised_when_server_rejects_registration() {
    let (mut client, _) = ClientHandler::new();
    let (request_id, _) = client.request_registration(ConnectionType::Tcp, 23);

    let response = ServerMessage::RegistrationFailed {
        request: request_id,
        cause: RegistrationFailureCause::PortAlreadyRegistered,
    };

    let results = client.handle_server_message(response).unwrap();

    assert_vec_contains!(results, ClientOperation::NotifyRegistrationFailed {request, cause}
    => {
        assert_eq!(*request, request_id, "Unexpected request id returned");
        assert_eq!(*cause, RegistrationFailureCause::PortAlreadyRegistered, "Unexpected cause");
    });
}

#[test]
fn error_returned_when_registration_failure_message_for_untracked_registration() {
    let (mut client, _) = ClientHandler::new();
    let (request_id, _) = client.request_registration(ConnectionType::Tcp, 23);

    let bad_request = RequestId(request_id.0 + 1);
    let response = ServerMessage::RegistrationFailed {
        request: bad_request,
        cause: RegistrationFailureCause::PortAlreadyRegistered,
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
    assert_vec_contains!(results, ClientOperation::CreateTcpConnectionForChannel {channel, new_connection}
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

#[test]
fn close_tcp_connection_operation_returned_when_dsrp_server_reports_closed_connection() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let connection1 = create_connection(&mut client, channel1);

    let message = ServerMessage::TcpConnectionClosed {
        channel: channel1,
        connection: connection1,
    };

    let results = client.handle_server_message(message).unwrap();
    assert_vec_contains!(results, ClientOperation::CloseTcpConnection {channel, connection}
    => {
        assert_eq!(*channel, channel1, "Unexpected channel identifier");
        assert_eq!(*connection, connection1, "Unexpected connection identifier");
    });
}

#[test]
fn no_operation_when_dsrp_server_reports_connection_closed_over_unknown_channel() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let connection1 = create_connection(&mut client, channel1);

    let message = ServerMessage::TcpConnectionClosed {
        channel: ChannelId(channel1.0 + 1),
        connection: connection1,
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

#[test]
fn no_operation_when_dsrp_server_reports_connection_closed_for_unknown_connection() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let connection1 = create_connection(&mut client, channel1);

    let message = ServerMessage::TcpConnectionClosed {
        channel: channel1,
        connection: ConnectionId(connection1.0 + 1),
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

#[test]
fn no_operation_when_dsrp_server_reports_connection_closed_for_non_owning_channel() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let channel2 = open_channel(&mut client, ConnectionType::Tcp, 24);
    let connection1 = create_connection(&mut client, channel1);

    let message = ServerMessage::TcpConnectionClosed {
        channel: channel2,
        connection: connection1,
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

#[test]
fn packet_relay_operation_returned_when_dsrp_reports_incoming_data_over_tcp_connection() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let connection1 = create_connection(&mut client, channel1);

    let expected_data = vec![1,2,3,4];
    let message = ServerMessage::DataReceived {
        channel: channel1,
        connection: Some(connection1),
        data: expected_data.clone(),
    };

    let results = client.handle_server_message(message).unwrap();
    assert_vec_contains!(results, ClientOperation::RelayRemotePacket {channel, connection, data}
    => {
        assert_eq!(*channel, channel1, "Unexpected channel identifier");
        assert_eq!(*connection, Some(connection1), "Unexpected connection identifier");
        assert_eq!(&data[..], &expected_data[..], "Unexpected data in packet");
    });
}

#[test]
fn packet_relay_operation_returned_when_dsrp_reports_incoming_data_over_udp_channel() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Udp, 23);

    let expected_data = vec![1,2,3,4];
    let message = ServerMessage::DataReceived {
        channel: channel1,
        connection: None,
        data: expected_data.clone(),
    };

    let results = client.handle_server_message(message).unwrap();
    assert_vec_contains!(results, ClientOperation::RelayRemotePacket {channel, connection, data}
    => {
        assert_eq!(*channel, channel1, "Unexpected channel identifier");
        assert_eq!(*connection, None, "Unexpected connection identifier");
        assert_eq!(&data[..], &expected_data[..], "Unexpected data in packet");
    });
}

#[test]
fn no_operation_when_data_received_message_for_unknown_channel() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Udp, 23);

    let expected_data = vec![1,2,3,4];
    let message = ServerMessage::DataReceived {
        channel: ChannelId(channel1.0 + 1),
        connection: None,
        data: expected_data.clone(),
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

#[test]
fn no_operation_when_data_received_message_for_unknown_connection() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let connection1 = create_connection(&mut client, channel1);

    let expected_data = vec![1,2,3,4];
    let message = ServerMessage::DataReceived {
        channel: channel1,
        connection: Some(ConnectionId(connection1.0 + 1)),
        data: expected_data.clone(),
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

#[test]
fn no_operation_when_data_received_message_for_connection_not_owned_by_channel() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let channel2 = open_channel(&mut client, ConnectionType::Tcp, 24);
    let connection1 = create_connection(&mut client, channel1);

    let expected_data = vec![1,2,3,4];
    let message = ServerMessage::DataReceived {
        channel: channel2,
        connection: Some(connection1),
        data: expected_data.clone(),
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

#[test]
fn no_operation_when_data_received_message_for_tcp_channel_without_connectionl() {
    let (mut client, _) = ClientHandler::new();
    let channel1 = open_channel(&mut client, ConnectionType::Tcp, 23);
    let _ = create_connection(&mut client, channel1);

    let expected_data = vec![1,2,3,4];
    let message = ServerMessage::DataReceived {
        channel: channel1,
        connection: None,
        data: expected_data.clone(),
    };

    let results = client.handle_server_message(message).unwrap();
    assert_eq!(results.len(), 0, "Unexpected number of operations returned");
}

fn open_channel(client: &mut ClientHandler, connection_type: ConnectionType, port: u16) -> ChannelId {
    let (request_id, _) = client.request_registration(connection_type, port);
    let channel = ChannelId(rand::random());
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

fn create_connection (client: &mut ClientHandler, channel: ChannelId) -> ConnectionId {
    let connection1 = ConnectionId(rand::random());
    let message = ServerMessage::NewIncomingTcpConnection {
        channel,
        new_connection: connection1,
    };

    let results = client.handle_server_message(message).unwrap();
    assert_vec_contains!(results, ClientOperation::CreateTcpConnectionForChannel {channel: notification_channel, new_connection}
    => {
        assert_eq!(*notification_channel, channel, "Unexpected channel identifier");
        assert_eq!(*new_connection, connection1, "Unexpected new connection identifier");
    });

    connection1
}