use pam_client::{Context, Flag};
use pam_client::conv_cli::Conversation;
use std::process::Command;
use std::os::unix::process::CommandExt;
use pwd_grp;

fn main() {
    let mut context = Context::new(
        // Service name (decides policy from /etc/pam.d)
        "su",
        // Optional preset username
        None,
        // Handler for user interaction
        Conversation::new()
    ).expect("Failed to initialize PAM context");

    // Optionally set some settings
    context.set_user_prompt(Some("Who art thou? ")).expect("Failed to prompt for username");

    // Authenticate the user (ask for password, 2nd factor, fingerprint, etc)
    context.authenticate(Flag::NONE).expect("Authentication failed");

    // Validate the account (is not locked, expired, etc)
    context.acct_mgmt(Flag::NONE).expect("Account validation failed");

    // Get resulting user name and map to a user id
    let username = context.user().expect("Unable to determine username");
    let passwd = pwd_grp::getpwnam(username).unwrap().unwrap();
    let uid = passwd.uid;

    println!("Trying to get session for uid {}", uid);

    // Open session and initialize credentials
    let session = context.open_session(Flag::NONE).expect("Session opening failed");

    // Run a process in the PAM environment
    let result = Command::new("/usr/bin/id")
                         .env_clear()
                         .envs(session.envlist().iter_tuples())
                         .uid(uid)
                      // .gid(...)
                         .status()
                         .expect("Process execution failed");

    // The session is automatically closed when it goes out of scope.

    println!("Result: {}", result);
}