/* TTY over QUIC - server
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

use std::{error::Error, path::Path, net::ToSocketAddrs};
use clap::Parser;
use s2n_quic::{Server, provider::io::tokio::Builder as IoBuilder, Connection, stream::BidirectionalStream};
use serde::Deserialize;
use tokio::{fs::read_to_string, io::copy_bidirectional};
use log::info;
use pty_process::{
    Command,
    Pty,
};

/// A Quic-based remote TTY
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Location of configuration file
    #[arg(short, long)]
    config: String,
}

#[derive(Deserialize)]
struct Config {
    cert_pem: String,
    key_pem: String,
    listen: String,
}

async fn pty(mut stream: BidirectionalStream, command: &mut Command) {
    let id = stream.connection().id();
    info!("[{}] Requesting PTY", id);

    let mut pty = Pty::new().unwrap();
    let mut child = command.spawn(&pty.pts().unwrap()).unwrap();

    match child.id() {
        Some(pid) => {
            info!("[{}] Spawned PID {}", id, pid);
        },
        None => { 
            info!("[{}] Child process had no PID...?", id);
        },
    }

    // This part actually handles the bidirectional stream, hopefully
    copy_bidirectional(&mut pty, &mut stream).await.unwrap();

    child.wait().await.unwrap();

    info!("[{}] Relinquishing PTY", id);
}

async fn handle_bidirectional_stream(stream: BidirectionalStream) {
    let id = stream.connection().id();
    info!("[{}] Received bidirectional stream", id);

    pty(stream, Command::new("/bin/login").arg("-h").arg("localhost")).await;

    info!("[{}] Closing bidirectional stream", id);
}

async fn handle_connection(mut connection: Connection) {
    let remote_addr = connection.remote_addr().unwrap();
    let id = connection.id();
    info!("[{}] Established Quic tunnel with remote address {}", id, remote_addr);

    // Accept bidirectional stream from client
    while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
        tokio::spawn(handle_bidirectional_stream(stream));
    }

    info!("[{}] Closing Quic tunnel", id);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("1");

    // Parse arguments
    let args = Args::parse();
    println!("2");

    // Get the dirname of the cfg file for relative pathing later
    // Current directory if no parent
    let relpath = match Path::new(&args.config).parent() {
        Some(path) => path,
        None => Path::new(".")
    };
    println!("3");

    // Load the entire configuration file and parse it
    let cfg: Config = toml::from_str(&read_to_string(&args.config).await?)?;
    println!("4");

    env_logger::init();
    info!("qtty 0.0.1");

    // Build a socket
    let io = IoBuilder::default()
        .with_receive_address(cfg.listen.to_socket_addrs()?.next().unwrap())?
        .build()?;

    // Start a Quic server
    let mut server = Server::builder()
        .with_tls((relpath.join(cfg.cert_pem).as_path(), relpath.join(cfg.key_pem).as_path()))?
        .with_io(io)?
        .start()?;

    info!("Listening");

    // Accept connections
    while let Some(connection) = server.accept().await {
        tokio::spawn(handle_connection(connection));
    }

    Ok(())
}
