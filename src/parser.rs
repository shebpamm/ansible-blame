use crate::entry::*;
use chrono::prelude::*;
use regex::Regex;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum LogParseError {
    #[error("Regex did not capture results")]
    RegexError,
    #[error("Could not parse time")]
    InvalidHost,
    #[error("Could not parse host")]
    InvalidService,
    #[error("Could not parse service")]
    InvalidTime,
}

fn parse_time(timestamp: &str) -> NaiveDateTime {
    // Add the current year to the timestamp so that chrono parses it correctly.
    let timestamp = format!("{} {}", Local::now().year(), timestamp);

    let time = NaiveDateTime::parse_from_str(&timestamp, "%Y %b %d %H:%M:%S").unwrap();
    time
}

pub fn get_ansible_runs(entries: Vec<LogEntry>) -> Vec<AnsibleRun> {
    let filtered_entries: Vec<LogEntry> = entries
        .into_iter()
        .filter(|entry| entry.service == Service::SUDO)
        .filter(|entry| entry.message.contains("AnsiballZ") || entry.message.contains("_=codecs.decode"))
        .collect();

    filtered_entries.into_iter().map(|entry| {
        let message = entry.message.split([':', ';']).map(|s| s.trim()).collect::<Vec<&str>>();
        let user = message[0];
        let strategy = match entry.message.contains("_=codecs.decode") {
            true => AnsibleStrategy::Mitogen,
            false => AnsibleStrategy::Native,
        };

        AnsibleRun {
            time: entry.time,
            host: entry.host,
            user: user.to_string(),
            strategy,
        }
    }).collect()
}

pub fn parse_lines(lines: Vec<String>) -> Vec<LogEntry> {
    lines
        .into_iter()
        .map(|line| parse_line(&line))
        .filter(|line| line.is_ok())
        .map(|line| line.unwrap())
        .collect::<Vec<LogEntry>>()
}

fn parse_line(line: &str) -> Result<LogEntry, LogParseError> {
    let time_regex = r"^[A-z][a-z]{2} \d{2} \d{2}:\d{2}:\d{2}";
    let host_regex = r"[a-zA-Z0-9-]+";
    let service_regex = r"[a-zA-Z0-9-]+";
    let message_regex = r".*$";

    let log_regex = format!(
        r"(?P<time>{})\s+(?P<host>{})\s+(?P<service>{})(?:\[[0-9]+\])?:\s+(?P<message>{})",
        time_regex, host_regex, service_regex, message_regex
    );

    let re = Regex::new(&log_regex).unwrap();

    let captures = re.captures(&line).unwrap();

    let time = captures
        .name("time")
        .ok_or(LogParseError::InvalidTime)?
        .as_str();
    let host = captures
        .name("host")
        .ok_or(LogParseError::InvalidHost)?
        .as_str();
    let service = captures
        .name("service")
        .ok_or(LogParseError::InvalidService)?
        .as_str();
    let message = captures
        .name("message")
        .ok_or(LogParseError::RegexError)?
        .as_str();

    let time = parse_time(time);
    let service = match Service::from_str(service) {
        Ok(service) => service,
        Err(_) => return Err(LogParseError::InvalidService),
    };

    Ok(LogEntry {
        time,
        service,
        host: host.to_string(),
        message: message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use chrono::prelude::*;

    #[test]
    fn parser_parses_time() {
        let current_year = Local::now().year();

        let line = "Jan 01 00:00:00 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash";
        let expected = NaiveDateTime::parse_from_str(&format!("{} Jan 01 00:00:00", current_year), "%Y %b %d %H:%M:%S").unwrap();

        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.time, expected);

        let line = "Mar 15 23:22:13 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash";
        let expected = NaiveDateTime::parse_from_str(&format!("{} Mar 15 23:22:13", current_year), "%Y %b %d %H:%M:%S").unwrap();

        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.time, expected);
    }

    #[test]
    fn parser_parses_host() {
        let line = "Jan 01 00:00:00 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash";
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.host, "host");
    }

    #[test]
    fn parser_parses_service() {
        let line = "Jan 01 00:00:00 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash";
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.service, super::Service::SUDO);

        let line = "Jan 01 00:00:00 host sshd[1234]: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash";
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.service, super::Service::SSHD);
    }

    #[test]
    fn parser_fails_unsupported_service() {
        let line = "Jan 01 00:00:00 host unsupported: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash";
        let result = super::parse_line(line).unwrap_err();
        let expected = super::LogParseError::InvalidService;
        assert_eq!(result, expected);
    }

    #[test]
    fn parser_parses_message() {
        let line = "Jan 01 00:00:00 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash";
        let entry = super::parse_line(line).unwrap();
        assert_eq!(
            entry.message,
            "user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash"
        );
    }
}
