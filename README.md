One struggle is that after building, you need to...

```
chmod 4755 target/debug/qtty
chown root:root target/debug/qtty
```

This is 100% folly. Things you never, ever, ever, ever do:

1. implement your own secure remote access system
2. suid root

And yet here we are. If you choose to expose this to the Internet, you deserve whatever foul
things happen to your server.

Resources:

PAM:
* https://docs.rs/pam-client/latest/pam_client/

PTY:
* https://superuser.com/questions/646491/what-is-the-difference-between-tty-and-vty-in-linux
* https://linux.die.net/man/4/pts
* http://www.linusakesson.net/programming/tty/

You have to open /dev/ptmx which gives you a file descriptor for a new PTY master, and creates
a new PTY slave in /dev/pts. The path to the new slave can be found by passing the master
descriptor to ptsname(3). Before opening the pty slave, you must pass the master's file
descriptor to grantpt(3) and unlockpt(3).

Once both are open, the slave provides processes with an interface that is identical to that
of a real terminal.

Data written to the slave is presented on the master descriptor as input. Data written to the
master is presented to the slave as input.

I asked ChatGPT about it and what it sounds like is...

1. qttyd communicates between the client over quic and the open /dev/ptmx file
2. After we get the /dev/pts assigned, we fork a new process that we have to create
3. Use `dup2` system call to duplicate file descriptor to the user's shell
4. Use `tcsetattr` system call to set terminal attributes such as line discipline and terminal modes (???)
5. Execute the shell binary using the `exec` family of functions, and this now takes control of the PTY
6. User interacts with qtty client, over quic, to qttyd, through /dev/ptmx to /dev/pts/n, into bash, into the kernel

ChatGPT recomments using `forkpty` which it sounds like is a higher level abstraction that will take care of this for us

Of course if anyone is ever going to use this, it has to not be Linux-specific. Support for the major BSD's
including Mac OS as well as ... however the hell you'd do this for Windows ... is essential.

HOW TO BUILD
============

Don't!

OK, if you insist, first you need the PAM development libraries

```
apt install libpam0g-dev

cargo build
```

For your convenience, there is a cert and key in this repo, and a cfg file for qttyd.
Obviously using this in production would be a horrible idea but doing it with that key
would just be criminally negligent.

You'll want to enable logging on the server and specify the config file:

```
RUST_LOG=debug ./target/debug/qttyd --config cfg.toml
```

Then you can try to connectg with the client, but the client is broken sadly.

```
qtty -c crt.pem localhost
```

I swear I had it working at one point. It looks like I tried to add support for killing
the connection and failed, and it don't work. Maybe I'll fix it someday. If you ctrl+c
the client, it won't die! You'll have to kill it.

TODO
====

Handle SIGWINCH on client and send info to server (maybe a separate bidirectional channel for this??)

Terminate gracefully

Make it like secure n stuff
