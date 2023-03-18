use openssh::{KnownHosts, Session};
use std::process::Output;
use thiserror::Error;

const AUTH_LOG_PATH: &str = "/var/log/auth.log";

#[derive(Error, Debug, PartialEq)]
enum RemoteLogError {
    #[error("Log file is not readable by user and Sudo is not available")]
    LogNotReadable,
}

async fn check_log_user_readable(
    session: &mut Session,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut cmd = session.command("test");
    cmd.arg("-r").arg(AUTH_LOG_PATH);

    Ok(cmd.status().await?.success())
}

async fn check_sudo_available(session: &mut Session) -> Result<bool, Box<dyn std::error::Error>> {
    let mut cmd = session.command("sudo");
    cmd.arg("-n");
    cmd.arg("true");
    Ok(cmd.status().await?.success())
}

fn parse_output(output: Output) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let contents = String::from_utf8(output.stdout)?;
    Ok(contents.lines().map(|s| s.to_string()).collect())
}

pub async fn read_remote_auth_log(host: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut session = Session::connect_mux(host, KnownHosts::Add).await?;

    let log_user_readable = check_log_user_readable(&mut session).await?;
    let sudo_available = check_sudo_available(&mut session).await?;

    // Go through the different scenarios where log reading requires sudo
    match (log_user_readable, sudo_available) {
        (true, _) => {
            let mut cmd = session.command("cat");
            cmd.arg(AUTH_LOG_PATH);
            let output = cmd.output().await?;
            parse_output(output)
        }
        (false, true) => {
            let mut cmd = session.command("sudo");
            cmd.arg("cat");
            cmd.arg(AUTH_LOG_PATH);
            let output = cmd.output().await?;
            parse_output(output)
        }
        (false, false) => Err(Box::new(RemoteLogError::LogNotReadable)),
    }
}
