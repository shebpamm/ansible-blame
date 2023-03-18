use chrono::prelude::*;
use strum::EnumString;

#[derive(Debug, PartialEq, EnumString)]
#[strum(serialize_all = "UPPERCASE")]
#[strum(ascii_case_insensitive)]
pub enum Service {
    CRON,
    SUDO,
    SSHD,
}

#[derive(Debug, PartialEq, EnumString)]
#[strum(serialize_all = "UPPERCASE")]
#[strum(ascii_case_insensitive)]
pub enum AnsibleStrategy {
    Native,
    Mitogen
}

#[derive(Debug)]
pub struct LogEntry {
    pub time: NaiveDateTime,
    pub host: String,
    pub service: Service,
    pub message: String,
}

#[derive(Debug)]
pub struct AnsibleRun {
    pub time: NaiveDateTime,
    pub host: String,
    pub user: String,
    pub strategy: AnsibleStrategy
}
