use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::io::{BufReader};
use futures::StreamExt;
use std::io;
use futures::io::ErrorKind;

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = "127.0.0.1:6142";
    let server = listen_for_dsrp_clients(addr);

    println!("DSRP server started running on {}", addr);
    server.await
}

async fn listen_for_dsrp_clients(addr: &str) -> io::Result<()> {
    let mut listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();
    while let Some(socket_result) = incoming.next().await {
        match socket_result {
            Err(err) => {
                println!("Accept error: {:?}", err);
            }

            Ok(socket) => {
                println!("Accepted connection from {:?}", socket.peer_addr()?);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(socket).await {
                        println!("An error occurred handling client: {:?}", e);
                    }
                });
            }
        }
    }

    Ok(())
}

async fn handle_client(mut socket: TcpStream) -> io::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);

    writer.write(b"Hello there!\n").await?;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("Client disconnected!");
                break;
            }

            Ok(_) => {writer.write(line.as_bytes()).await?;},
            Err(e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(e) if e.kind() == ErrorKind::Interrupted => (),
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            },
        }
    }

    Ok(())
}