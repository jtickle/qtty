/* Experimenting with QUIC - just the transport - server
 * Copyright (C) 2025 Jeffrey W. Tickle
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use s2n_quic::Server;
use tokio::time::sleep;
use std::{
    error::Error,
    sync::{Arc, Mutex}, time::Duration
};

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
    let thread_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    let mut server = Server::builder()
        .with_tls((CERT_PEM, KEY_PEM))?
        .with_io("127.0.0.1:2222")?
        .start()?;

    let outside_count = Arc::clone(&thread_count);

    tokio::spawn(async move {
        loop {
            let n = *outside_count.lock().unwrap();
            eprintln!("Task Count: {}", n);
            sleep(Duration::from_secs(5)).await;
        }
    });

    while let Some(mut connection) = server.accept().await {
        let concount = Arc::clone(&thread_count);
        // spawn a new task for the connection
        tokio::spawn(async move {
            *concount.lock().unwrap() += 1;
            let addr = connection.remote_addr();
            eprintln!("Connection accepted from {:?}", connection.remote_addr());

            while let Ok(Some(mut stream)) = connection.accept_bidirectional_stream().await {
                let streamcount = Arc::clone(&concount);
                // spawn a new task for the stream
                tokio::spawn(async move {
                    *streamcount.lock().unwrap() += 1;
                    eprintln!("Stream opened from {:?}", addr);

                    // echo any data back to the stream
                    // TODO: interact with the ptmx
                    while let Ok(Some(data)) = stream.receive().await {
                        eprint!("Echoing: {}", String::from_utf8_lossy(&data));
                        stream.send(data).await.expect("stream should be open");
                    }

                    eprintln!("Stream closed from {:?}", addr);
                    *streamcount.lock().unwrap() -= 1;
                });
            }

            eprintln!("Connection closed from {:?}", addr);
            *concount.lock().unwrap() -= 1;
        });
    }

    Ok(())
}
