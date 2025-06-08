/* Experiment with PTY virtual teletype system in Rust
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

use pty_process::{
    Command,
    Pty,
};
use termios::{Termios, tcsetattr};
use tokio::io::{stdin, stdout, AsyncReadExt, AsyncWriteExt};
use std::{error::Error, os::fd::AsRawFd};

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
    let pty = Pty::new()?;
    let mut cmd = Command::new("/bin/bash");
    let mut child = cmd.spawn(&pty.pts()?)?;

    match child.id() {
        Some(id) => eprintln!("Spawned PID {}", id),
        None => eprintln!("Child process had no ID...?"),
    }

    let (mut child_read, mut child_write) = pty.into_split();

    // Disable local echo
    let stdin_fd = stdin().as_raw_fd();
    set_term_attrs(stdin_fd);

    // Input
    tokio::spawn(async move {

        let mut input = stdin();
        let mut buffer:[u8;4096] = [0; 4096];

        while let Ok(n) = input.read(&mut buffer).await {
            child_write.write(&buffer[0 .. n]).await.unwrap();
        }
    });

    // Output
    tokio::spawn(async move {

        let mut output = stdout();
        let mut buffer:[u8;4096] = [0; 4096];

        while let Ok(n) = child_read.read(&mut buffer).await {
            output.write(&buffer[0 .. n]).await.unwrap();
            output.flush().await.unwrap();
        }
    });

    child.wait().await?;

    reset_term_attrs(stdin_fd);

    println!("Press enter to terminate...");

    Ok(())
}
