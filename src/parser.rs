use crate::entry::{LogEntry, Service};
use regex::Regex;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Regex did not capture results")]
    RegexError,
    #[error("Could not parse time")]
    InvalidHost,
    #[error("Could not parse host")]
    InvalidService,
    #[error("Could not parse service")]
    InvalidTime,
}

pub fn parse_line(line: String) -> Result<LogEntry, ParseError> {
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
        .ok_or(ParseError::InvalidTime)?
        .as_str();
    let host = captures
        .name("host")
        .ok_or(ParseError::InvalidHost)?
        .as_str();
    let service = match Service::from_str(
        captures
            .name("service")
            .ok_or(ParseError::InvalidService)?
            .as_str(),
    ) {
        Ok(service) => service,
        Err(_) => return Err(ParseError::InvalidService),
    };
    let message = captures
        .name("message")
        .ok_or(ParseError::RegexError)?
        .as_str();

    Ok(LogEntry {
        time: time.to_string(),
        host: host.to_string(),
        service,
        message: message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn parser_parses_time() {
        let line = "Jan 01 00:00:00 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash".to_string();
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.time, "Jan 01 00:00:00");

        let line = "Mar 15 23:22:13 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash".to_string();
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.time, "Mar 15 23:22:13");
    }

    #[test]
    fn parser_parses_host() {
        let line = "Jan 01 00:00:00 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash".to_string();
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.host, "host");
    }

    #[test]
    fn parser_parses_service() {
        let line = "Jan 01 00:00:00 host sudo: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash".to_string();
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.service, super::Service::SUDO);

        let line = "Jan 01 00:00:00 host sshd[1234]: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash".to_string();
        let entry = super::parse_line(line).unwrap();
        assert_eq!(entry.service, super::Service::SSHD);
    }

    #[test]
    fn parser_fails_unsupported_service() {
        let line = "Jan 01 00:00:00 host unsupported: user : TTY=pts/0 ; PWD=/home/user ; USER=root ; COMMAND=/bin/bash".to_string();
        let result = super::parse_line(line).unwrap_err();
        let expected = super::ParseError::InvalidService;
        assert_eq!(result, expected);
    }
}
