use std::{error::Error, net::ToSocketAddrs, path::Path, os::fd::AsRawFd};
use clap::Parser;
use s2n_quic::{Client, provider::io::tokio::Builder as IoBuilder, client::Connect};
use termios::{Termios, tcsetattr};
use tokio::io::{stdout, copy, stdin};

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