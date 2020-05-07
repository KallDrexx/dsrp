use tokio::net::{TcpStream};
use tokio::prelude::*;
use tokio::io::{BufReader};
use std::io;
use futures::io::ErrorKind;

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = "127.0.0.1:6142";
    let server = connect_to_server(addr);

    println!("DSRP server started running on {}", addr);
    server.await
}

async fn connect_to_server(addr: &str) -> io::Result<()> {
    let mut stream = TcpStream::connect(addr).await?;
    println!("Connected to server!");

    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("Connection disconnected!");
                break;
            }

            Ok(_) => {
                println!("Received: {}", line);
                writer.write(line.as_bytes()).await?;
            }

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