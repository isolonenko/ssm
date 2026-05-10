use crate::config::Host;
use std::path::{Path, PathBuf};

pub fn parse_ssh_config(path: &Path) -> Vec<Host> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut hosts = Vec::new();
    let mut current_alias: Option<String> = None;
    let mut hostname = String::new();
    let mut user: Option<String> = None;
    let mut port: u16 = 22;
    let mut identity_file: Option<PathBuf> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Host ").or_else(|| trimmed.strip_prefix("Host\t"))
        {
            // Flush previous host
            if let Some(alias) = current_alias.take() {
                if !hostname.is_empty() && alias != "*" {
                    hosts.push(Host {
                        alias,
                        hostname: hostname.clone(),
                        user: user.take(),
                        port,
                        identity_file: identity_file.take(),
                        tags: vec![],
                        notes: None,
                        tunnels: vec![],
                        commands: vec![],
                    });
                }
            }

            let alias = rest.trim().to_string();
            current_alias = Some(alias);
            hostname = String::new();
            user = None;
            port = 22;
            identity_file = None;
        } else if current_alias.is_some() {
            let (key, value) = match trimmed.split_once(char::is_whitespace) {
                Some((k, v)) => (k.to_lowercase(), v.trim().to_string()),
                None => continue,
            };

            match key.as_str() {
                "hostname" => hostname = value,
                "user" => user = Some(value),
                "port" => port = value.parse().unwrap_or(22),
                "identityfile" => identity_file = Some(PathBuf::from(value)),
                _ => {}
            }
        }
    }

    // Flush last host
    if let Some(alias) = current_alias {
        if !hostname.is_empty() && alias != "*" {
            hosts.push(Host {
                alias,
                hostname,
                user,
                port,
                identity_file,
                tags: vec![],
                notes: None,
                tunnels: vec![],
                commands: vec![],
            });
        }
    }

    hosts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_basic_hosts() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
Host prod-api
    HostName 10.0.1.50
    User deploy
    Port 2222
    IdentityFile ~/.ssh/id_ed25519

Host staging
    HostName staging.example.com
    User admin
"#
        )
        .unwrap();

        let hosts = parse_ssh_config(f.path());
        assert_eq!(hosts.len(), 2);

        assert_eq!(hosts[0].alias, "prod-api");
        assert_eq!(hosts[0].hostname, "10.0.1.50");
        assert_eq!(hosts[0].user, Some("deploy".to_string()));
        assert_eq!(hosts[0].port, 2222);
        assert_eq!(
            hosts[0].identity_file,
            Some(PathBuf::from("~/.ssh/id_ed25519"))
        );

        assert_eq!(hosts[1].alias, "staging");
        assert_eq!(hosts[1].hostname, "staging.example.com");
        assert_eq!(hosts[1].user, Some("admin".to_string()));
        assert_eq!(hosts[1].port, 22);
    }

    #[test]
    fn test_skips_wildcard_and_no_hostname() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
Host *
    ServerAliveInterval 60

Host no-hostname
    User test

Host valid
    HostName 1.2.3.4
"#
        )
        .unwrap();

        let hosts = parse_ssh_config(f.path());
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "valid");
    }

    #[test]
    fn test_nonexistent_file_returns_empty() {
        let hosts = parse_ssh_config(Path::new("/tmp/nonexistent-ssh-config-12345"));
        assert!(hosts.is_empty());
    }
}
