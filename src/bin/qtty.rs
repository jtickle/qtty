/* TTY over QUIC - client
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

use std::{error::Error, net::ToSocketAddrs, path::Path, os::fd::AsRawFd};
use clap::Parser;
use s2n_quic::{Client, provider::io::tokio::Builder as IoBuilder, client::Connect};
use termios::{Termios, tcsetattr};
use tokio::io::{stdout, copy, stdin};
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg()]
    remote: String,
    #[arg(short, default_value("2222"))]
    port: u16,
    #[arg(short)]
    ca: String,
}

fn set_term_attrs(fd: i32) {
    let mut termstruct = Termios::from_fd(fd).unwrap();
    termstruct.c_lflag &= !(termios::ICANON | termios::ECHO);
    tcsetattr(fd, termios::TCSADRAIN, &termstruct).unwrap();
}

fn reset_term_attrs(fd: i32) {
    let mut termstruct = Termios::from_fd(fd).unwrap();
    termstruct.c_lflag |= termios::ICANON | termios::ECHO;
    tcsetattr(fd, termios::TCSAFLUSH, &termstruct).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    // Track all created tasks
    let tracker = TaskTracker::new();

    // This will let us communicate cancellation
    let cancel_token = CancellationToken::new();

    // Handle signals
    let c_c_cancel_token = cancel_token.clone();
    tracker.spawn(async move {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("Terminating due to Ctrl+C");
                c_c_cancel_token.cancel();
            }
            _ = c_c_cancel_token.cancelled() => {
                println!("Task was cancelled through other means");
            }
        }
    });

    // Handle signals
    match signal::ctrl_c().await {
        Ok(()) => {
            println!("Ctrl+C Pressed");
        },
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Parse arguments
    let args = Args::parse();

    // Get remote address
    let addr = (args.remote.to_owned(), args.port).to_socket_addrs()?.next().unwrap();

    // Build Quic client
    let client = Client::builder()
        .with_tls(Path::new(&args.ca))?
        .with_io("0.0.0.0:0")?
        .start()?;

    let connect = Connect::new(addr).with_server_name(args.remote.to_owned());
    let mut connection = client.connect(connect).await?;

    // ensure connection doesn't time out with inactivity
    connection.keep_alive(true)?;

    let stream = connection.open_bidirectional_stream().await?;
    let (mut receive_stream, mut send_stream) = stream.split();

    let receive_task = tokio::spawn(async move {
        let mut stdout = stdout();
        let _ = copy(&mut receive_stream, &mut stdout).await;
    });

    // Disable local echo
    let stdin_fd = stdin().as_raw_fd();
    set_term_attrs(stdin_fd);

    // Copy data from stdin and send it to the server
    let mut stdin = stdin();
    copy(&mut stdin, &mut send_stream).await?;

    println!("Before abort");

    receive_task.abort();

    reset_term_attrs(stdin_fd);

    println!("After abort");

    Ok(())
}
