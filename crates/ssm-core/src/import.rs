use crate::config::Host;
use std::collections::HashMap;
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

// ---------------------------------------------------------------------------
// SSH command paste import
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTunnel {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedHost {
    pub hostname: String,
    pub user: Option<String>,
    pub port: u16,
    pub tunnels: Vec<ParsedTunnel>,
}

/// Parse raw text containing ssh commands (one per line) and group tunnels by host.
/// Handles formats like:
///   ssh -l ivan -nNTL 8881:localhost:8881 ash.onomatics.com -p 10010
///   ssh -L 5432:localhost:5432 -N user@host
pub fn parse_ssh_commands(text: &str) -> Vec<ParsedHost> {
    let mut host_map: HashMap<(String, u16), ParsedHost> = HashMap::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.contains("ssh ") && !trimmed.starts_with("ssh ") {
            continue;
        }

        // Extract just the ssh command part (skip any prefix like echo, etc.)
        let ssh_part = if let Some(idx) = trimmed.find("ssh ") {
            &trimmed[idx..]
        } else {
            continue;
        };

        if let Some(parsed) = parse_single_ssh_command(ssh_part) {
            let key = (parsed.hostname.clone(), parsed.port);
            let entry = host_map.entry(key).or_insert_with(|| ParsedHost {
                hostname: parsed.hostname.clone(),
                user: parsed.user.clone(),
                port: parsed.port,
                tunnels: vec![],
            });
            entry.tunnels.extend(parsed.tunnels);
        }
    }

    let mut hosts: Vec<ParsedHost> = host_map.into_values().collect();
    hosts.sort_by(|a, b| a.hostname.cmp(&b.hostname));
    hosts
}

#[derive(Debug)]
struct ParsedCommand {
    hostname: String,
    user: Option<String>,
    port: u16,
    tunnels: Vec<ParsedTunnel>,
}

fn parse_single_ssh_command(cmd: &str) -> Option<ParsedCommand> {
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    if tokens.is_empty() || tokens[0] != "ssh" {
        return None;
    }

    let mut user: Option<String> = None;
    let mut port: u16 = 22;
    let mut tunnels: Vec<ParsedTunnel> = Vec::new();
    let mut hostname: Option<String> = None;
    let mut i = 1;

    while i < tokens.len() {
        let tok = tokens[i];

        if tok == "-l" {
            i += 1;
            if i < tokens.len() {
                user = Some(tokens[i].to_string());
            }
        } else if tok == "-p" {
            i += 1;
            if i < tokens.len() {
                port = tokens[i].parse().unwrap_or(22);
            }
        } else if tok == "-L" {
            i += 1;
            if i < tokens.len() {
                if let Some(t) = parse_tunnel_spec(tokens[i]) {
                    tunnels.push(t);
                }
            }
        } else if tok.starts_with('-') {
            // Combined flags like -nNTL — check if L is in there
            if let Some(after_l) = tok.strip_suffix('L').or_else(|| {
                // L might be followed by nothing (next token is the spec)
                if tok.contains('L') {
                    // -nNTL or -NL etc — L is at the end taking next arg
                    let l_pos = tok.rfind('L')?;
                    if l_pos == tok.len() - 1 {
                        Some(&tok[..l_pos])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                let _ = after_l;
                i += 1;
                if i < tokens.len() {
                    if let Some(t) = parse_tunnel_spec(tokens[i]) {
                        tunnels.push(t);
                    }
                }
            }
            // Other flags we don't care about (-n, -N, -T, -f, etc.)
        } else if tok.contains('@') {
            // user@host format
            let parts: Vec<&str> = tok.splitn(2, '@').collect();
            if parts.len() == 2 {
                user = Some(parts[0].to_string());
                hostname = Some(parts[1].to_string());
            }
        } else if tok.ends_with('&') {
            // trailing & from backgrounding — skip
        } else {
            // Positional argument = hostname
            hostname = Some(tok.to_string());
        }

        i += 1;
    }

    let hostname = hostname?;
    if tunnels.is_empty() {
        return None;
    }

    Some(ParsedCommand {
        hostname,
        user,
        port,
        tunnels,
    })
}

fn parse_tunnel_spec(spec: &str) -> Option<ParsedTunnel> {
    // Format: local_port:remote_host:remote_port
    let parts: Vec<&str> = spec.splitn(3, ':').collect();
    if parts.len() == 3 {
        let local_port = parts[0].parse().ok()?;
        let remote_host = parts[1].to_string();
        let remote_port = parts[2].parse().ok()?;
        Some(ParsedTunnel {
            local_port,
            remote_host,
            remote_port,
        })
    } else {
        None
    }
}

/// Generate a short alias from a hostname (e.g. "ash.onomatics.com" → "ash")
pub fn alias_from_hostname(hostname: &str) -> String {
    hostname
        .split('.')
        .next()
        .unwrap_or(hostname)
        .to_string()
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

    #[test]
    fn test_parse_ssh_commands_basic() {
        let input = r#"
ssh -l ivan -nNTL 8881:localhost:8881 ash.onomatics.com -p 10010 &
ssh -l ivan -nNTL 8700:localhost:8700 ash.onomatics.com -p 10010 &
ssh -l ivan -nNTL 7213:localhost:7213 mediaservice-dev.onomatics.com -p 10010 &
"#;
        let hosts = parse_ssh_commands(input);
        assert_eq!(hosts.len(), 2);

        let ash = hosts.iter().find(|h| h.hostname == "ash.onomatics.com").unwrap();
        assert_eq!(ash.port, 10010);
        assert_eq!(ash.user, Some("ivan".to_string()));
        assert_eq!(ash.tunnels.len(), 2);
        assert!(ash.tunnels.iter().any(|t| t.local_port == 8881));
        assert!(ash.tunnels.iter().any(|t| t.local_port == 8700));

        let media = hosts.iter().find(|h| h.hostname == "mediaservice-dev.onomatics.com").unwrap();
        assert_eq!(media.tunnels.len(), 1);
        assert_eq!(media.tunnels[0].local_port, 7213);
    }

    #[test]
    fn test_parse_ssh_commands_user_at_host() {
        let input = "ssh -NL 5432:localhost:5432 deploy@db.example.com";
        let hosts = parse_ssh_commands(input);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname, "db.example.com");
        assert_eq!(hosts[0].user, Some("deploy".to_string()));
        assert_eq!(hosts[0].tunnels[0].local_port, 5432);
    }

    #[test]
    fn test_parse_ssh_commands_skips_non_tunnel() {
        let input = r#"
echo "Starting tunnels"
ssh user@host.com
ssh -l ivan -nNTL 8881:localhost:8881 ash.com -p 10010
echo "done"
"#;
        let hosts = parse_ssh_commands(input);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname, "ash.com");
    }

    #[test]
    fn test_alias_from_hostname() {
        assert_eq!(alias_from_hostname("ash.onomatics.com"), "ash");
        assert_eq!(alias_from_hostname("10.0.1.50"), "10");
        assert_eq!(alias_from_hostname("myhost"), "myhost");
    }
}
