use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token, Registry};
use mio::event::Event;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::str::from_utf8;

const SERVER: Token = Token(0);

fn main() -> io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut connections = HashMap::new();
    let mut next_token_id = 1;

    let addr = "127.0.0.1:9999".parse().unwrap();
    let mut server = TcpListener::bind(addr)?;

    // Start listening for incoming connections
    poll.registry().register(&mut server, SERVER, Interest::READABLE)?;

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    // Event for the server, which means a connection is ready to be accepted
                    let (mut connection, address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // Wouldblock means no more connections are queued, so go back to polling
                            break;
                        },

                        Err(e) => {
                            return Err(e);
                        }
                    };

                    println!("Accepted connection from: {}", address);

                    let client_token = Token(next_token_id);
                    next_token_id += 1;

                    poll.registry()
                        .register(
                            &mut connection,
                            client_token,
                            Interest::READABLE.add(Interest::WRITABLE))?;

                    connections.insert(client_token, connection);
                }

                token => {
                    // Event for a specific client
                    let client_finished = if let Some(connection) = connections.get_mut(&token) {
                        // handle event
                        handle_client_event(poll.registry(), connection, event)?
                    } else {
                        // Sometimes events fire without a valid client apparently, so ignore these
                        false
                    };

                    if client_finished {
                        connections.remove(&token);
                    }
                }
            }
        }
    }
}

fn handle_client_event(registry: &Registry, connection: &mut TcpStream, event: &Event) -> io::Result<bool> {
    const DATA: &[u8] = b"Hello world!\n";

    if event.is_writable() {
        match connection.write(DATA) {
            Ok(x) if x < DATA.len() => return Err(io::ErrorKind::WriteZero.into()),
            Ok(_) => {
                // We wrote all the bytes, and we don't want to write them again, so poll for read only
                registry.reregister(connection, event.token(), Interest::READABLE)?;
            }

            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                return handle_client_event(registry, connection, event);
            }

            Err(err) => {
                return Err(err)
            }
        }
    }

    if event.is_readable() {
        let mut should_close_connection = false;
        let mut received_data = Vec::with_capacity(4096);
        loop {
            let mut buf = [0; 256];
            match connection.read(&mut buf) {
                Ok(0) => {
                    should_close_connection = true;
                    break;
                }

                Ok(n) => received_data.extend_from_slice(&buf[..n]),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }

        if let Ok(str_buf) = from_utf8(&received_data) {
            println!("Received data: {}", str_buf);
        } else {
            println!("Received non-UTF-8 data: {:?}", &received_data);
        }

        if should_close_connection {
            println!("Closing connection");
            return Ok(true);
        }
    }

    Ok(false)
}