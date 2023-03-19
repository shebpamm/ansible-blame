use openssh::{KnownHosts, Session};
use std::process::Output;
use thiserror::Error;

const DEBIAN_AUTH_LOG_PATH: &str = "/var/log/auth.log";
const REDHAT_AUTH_LOG_PATH: &str = "/var/log/secure";

#[derive(Error, Debug, PartialEq)]
enum RemoteLogError {
    #[error("Log file is not readable by user and sudo is not available")]
    LogNotReadable,
    #[error("Unsupported distribution")]
    UnsupportedDistro,
}

async fn check_distribution(session: &mut Session) -> anyhow::Result<String> {
    let mut cmd = session.command("cat");
    cmd.arg("/etc/os-release");
    let output = cmd.output().await?;
    let contents = String::from_utf8(output.stdout)?;
    let lines = contents.lines().collect::<Vec<&str>>();

    // Find line starting with ID=
    let name_line = lines
        .into_iter()
        .find(|line| line.starts_with("ID="))
        .ok_or(RemoteLogError::UnsupportedDistro)?;

    // Remove the "ID=" part and any quotes and return the rest
    let name = name_line
        .replace("ID=", "")
        .replace("\"", "")
        .replace("'", "");
    Ok(name)

}

async fn get_log_path(session: &mut Session) -> anyhow::Result<String> {
    let distribution = check_distribution(session).await?;
    match distribution.as_str() {
        "debian" | "ubuntu" => Ok(DEBIAN_AUTH_LOG_PATH.to_string()),
        "centos" | "rhel" => Ok(REDHAT_AUTH_LOG_PATH.to_string()),
        _ => Err(RemoteLogError::UnsupportedDistro.into()),
    }
}

async fn check_log_user_readable(
    session: &mut Session,
    log_path: &str,
) -> anyhow::Result<bool> {
    let mut cmd = session.command("test");
    cmd.arg("-r").arg(log_path);

    Ok(cmd.status().await?.success())
}

async fn check_sudo_available(session: &mut Session) -> anyhow::Result<bool> { 
    let mut cmd = session.command("sudo");
    cmd.arg("-n");
    cmd.arg("true");
    Ok(cmd.status().await?.success())
}

fn parse_output(output: Output) -> anyhow::Result<Vec<String>> {
    let contents = String::from_utf8(output.stdout)?;
    Ok(contents.lines().map(|s| s.to_string()).collect())
}

pub async fn read_remote_auth_log(host: &str) -> anyhow::Result<Vec<String>> {
    let mut session = Session::connect_mux(host, KnownHosts::Add).await?;

    let log_path = get_log_path(&mut session).await?;

    let log_user_readable = check_log_user_readable(&mut session, &log_path).await?;
    let sudo_available = check_sudo_available(&mut session).await?;

    // Go through the different scenarios where log reading requires sudo
    match (log_user_readable, sudo_available) {
        (true, _) => {
            let mut cmd = session.command("cat");
            cmd.arg(log_path);
            let output = cmd.output().await?;
            parse_output(output)
        }
        (false, true) => {
            let mut cmd = session.command("sudo");
            cmd.arg("cat");
            cmd.arg(log_path);
            let output = cmd.output().await?;
            parse_output(output)
        }
        (false, false) => Err(RemoteLogError::LogNotReadable.into()),
    }
}
