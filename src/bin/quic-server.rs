use s2n_quic::Server;
use std::error::Error;

pub static CERT_PEM: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/crt.pem"
));
pub static KEY_PEM: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/key.pem"
));

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut server = Server::builder()
        .with_tls((CERT_PEM, KEY_PEM))?
        .with_io("127.0.0.1:2222")?
        .start()?;

    while let Some(mut connection) = server.accept().await {
        // spawn a new task for the connection
        tokio::spawn(async move {
            let addr = connection.remote_addr();
            eprintln!("Connection accepted from {:?}", connection.remote_addr());

            while let Ok(Some(mut stream)) = connection.accept_bidirectional_stream().await {
                // spawn a new task for the stream
                tokio::spawn(async move {
                    eprintln!("Stream opened from {:?}", addr);

                    // echo any data back to the stream
                    // TODO: interact with the ptmx
                    while let Ok(Some(data)) = stream.receive().await {
                        if data[0] == 4 {
                            eprintln!("EOF detected");
                            break;
                        }
                        eprint!("Echoing: {}", String::from_utf8_lossy(&data));
                        stream.send(data).await.expect("stream should be open");
                    }

                    eprintln!("Stream closed from {:?}", addr)
                });
            }

            eprintln!("Connection closed from {:?}", addr);
        });
    }

    Ok(())
}