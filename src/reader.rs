use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use openssh::{KnownHosts, Session, Stdio};
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
    #[error("Failed to open stdin for sudo")]
    SudoStdin,
}

pub enum SourceReader {
    Local(LocalSource),
    Remote(RemoteSource),
}

#[async_trait]
trait Readable {
    async fn read(&self) -> anyhow::Result<Vec<String>>;
}

pub struct LocalSource {
    pub path: PathBuf,
}

impl LocalSource {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl Readable for LocalSource {
    async fn read(&self) -> anyhow::Result<Vec<String>> {
        let contents = fs::read_to_string(&self.path).await?;
        Ok(contents.lines().map(|s| s.to_string()).collect())
    }
}

pub struct RemoteSource {
    pub host: String,
    pub password: Option<String>,
}

impl RemoteSource {
    pub fn new(host: String, password: Option<String>) -> Self {
        Self { host, password }
    }

    async fn get_log_path(&self, session: &mut Session) -> anyhow::Result<String> {
        let distribution = self.check_distribution(session).await?;
        match distribution.as_str() {
            "debian" | "ubuntu" => Ok(DEBIAN_AUTH_LOG_PATH.to_string()),
            "centos" | "rhel" => Ok(REDHAT_AUTH_LOG_PATH.to_string()),
            _ => Err(RemoteLogError::UnsupportedDistro.into()),
        }
    }

    async fn check_distribution(&self, session: &mut Session) -> anyhow::Result<String> {
        let mut cmd = session.command("cat");
        cmd.arg("/etc/os-release");
        cmd.stderr(Stdio::null());
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

    async fn check_log_user_readable(
        &self,
        session: &mut Session,
        log_path: &str,
    ) -> anyhow::Result<bool> {
        let mut cmd = session.command("test");
        cmd.arg("-r").arg(log_path);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        Ok(cmd.status().await?.success())
    }

    async fn check_sudo_available(&self, session: &mut Session) -> anyhow::Result<bool> {
        let mut cmd = session.command("sudo");
        cmd.arg("-n");
        cmd.arg("true");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());
        let sudo_requires_password = !cmd.status().await?.success();

        // If sudo doesn't require a password, return true
        if !sudo_requires_password {
            return Ok(true);
        }

        // If sudo requires a password, check if we have one
        if self.password == None {
            return Ok(false);
        } else {
            return Ok(true);
        }
    }

    fn parse_output(&self, output: Output) -> anyhow::Result<Vec<String>> {
        let contents = String::from_utf8(output.stdout)?;
        Ok(contents.lines().map(|s| s.to_string()).collect())
    }
}

#[async_trait]
impl Readable for RemoteSource {
    async fn read(&self) -> anyhow::Result<Vec<String>> {
        let mut session = Session::connect_mux(&self.host, KnownHosts::Add).await?;

        let log_path = self.get_log_path(&mut session).await?;

        let log_user_readable = self
            .check_log_user_readable(&mut session, &log_path)
            .await?;
        let sudo_available = self.check_sudo_available(&mut session).await?;

        // Go through the different scenarios where log reading requires sudo
        match (log_user_readable, sudo_available) {
            (true, _) => {
                let mut cmd = session.command("cat");
                cmd.arg(log_path);
                let output = cmd.output().await?;
                self.parse_output(output)
            }
            (false, true) => {
                let mut cmd = session.command("sudo");
                cmd.arg("-S");
                cmd.arg("cat");
                cmd.arg(log_path);
                let output = match self.password {
                    // If we have a password, pipe it to sudo
                    Some(ref password) => {
                        cmd.stdin(Stdio::piped());
                        cmd.stderr(Stdio::null());
                        let mut child = cmd.spawn().await?;

                        let child_stdin = child.stdin().as_mut().ok_or(RemoteLogError::SudoStdin)?;
                        child_stdin.write_all(password.as_bytes()).await?;
                        drop(child_stdin);

                        let output = child.wait_with_output().await?; // No idea why it's a () if I
                                                                      // don't put it in a variable
                        output
                    }
                    None => cmd.output().await?,
                };
                self.parse_output(output)
            }
            (false, false) => Err(RemoteLogError::LogNotReadable.into()),
        }
    }
}

impl SourceReader {
    pub async fn read(&self) -> anyhow::Result<Vec<String>> {
        match self {
            SourceReader::Local(source) => source.read().await,
            SourceReader::Remote(source) => source.read().await,
        }
    }
}
