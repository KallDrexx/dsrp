use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use std::io::{self, Read, Write};
use std::str::from_utf8;

const SERVER: Token = Token(0);

fn main() {
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(128);

    let addr = "127.0.0.1:9999".parse().unwrap();

    let mut dsrp_server_stream = TcpStream::connect(addr).unwrap();
    poll.registry()
        .register(
            &mut dsrp_server_stream,
            SERVER,
            Interest::READABLE.add(Interest::WRITABLE))
        .unwrap();

    println!("Attempting to connect to server on port 9999");
    let mut sent_data = false;
    let mut total_received_bytes = 0;

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    if event.is_writable() {
                        const DATA: &[u8] = b"Hello server!\n";

                        if !sent_data {
                            match dsrp_server_stream.write(DATA) {
                                Ok(x) if x < DATA.len() => panic!("Couldn't write full data to stream"),
                                Ok(_) => sent_data = true,
                                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {}
                                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {}
                                Err(err) => panic!("{:?}", err),
                            }
                        }
                    }

                    if event.is_readable() {
                        let mut connection_closed = false;
                        let mut received_data = Vec::with_capacity(4096);
                        loop {
                            let mut buf = [0; 256];
                            match dsrp_server_stream.read(&mut buf) {
                                Ok(0) => {
                                    connection_closed = true;
                                    break;
                                }

                                Ok(n) => {
                                    received_data.extend_from_slice(&buf[..n]);
                                    total_received_bytes += n;
                                },
                                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {},
                                Err(err) => panic!("{:?}", err),
                            }
                        }

                        if let Ok(str_buf) = from_utf8(&received_data) {
                            println!("Received data: {}", str_buf);
                        } else {
                            println!("Received non-UTF-8 data: {:?}", &received_data);
                        }

                        if connection_closed || total_received_bytes >= 13 {
                            println!("Finished!");
                            return;
                        }
                    }
                }

                _ => {
                    // shouldn't happen yet
                }
            }
        }
    }
}
