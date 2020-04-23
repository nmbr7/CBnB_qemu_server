mod api;
mod message;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};

use futures::future::try_join;
use futures::FutureExt;
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    // TODO Parse the msg for and run corresponding operations

    // TODO lookp the docker ip from based of the app uuid
    let server_addr = String::from("127.0.0.1:8888");
    let listen_addr = String::from("127.0.0.1:8080");
    println!("Listening on: {}", listen_addr);
    println!("Proxying to: {}", server_addr);

    let mut listener = TcpListener::bind(listen_addr).await?;

    while let Ok((inbound, _)) = listener.accept().await {
        let d_conn = docker_conn(inbound, server_addr.clone()).map(|r| {
            if let Err(e) = r {
                println!("Failed to Connect to docker app ; error={}", e);
            }
        });
        tokio::spawn(d_conn);
    }
    Ok(())
}


async fn create_app(app_storage_id: String) -> Result<(), Box<dyn Error>> {

    // TODO Fetch the app files from cloud storage and unzip ( Use the download function from the cbnb cli )

    // TODO Create a container from the yaml or the corresponding config file

    Ok(())
}

async fn docker_conn(mut inbound: TcpStream, proxy_addr: String) -> Result<(), Box<dyn Error>> {
    let mut outbound = TcpStream::connect(proxy_addr).await?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = io::copy(&mut ri, &mut wo);
    let server_to_client = io::copy(&mut ro, &mut wi);

    try_join(client_to_server, server_to_client).await?;

    Ok(())
}
