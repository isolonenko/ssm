use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unresolved placeholder: '{0}'")]
    UnresolvedPlaceholder(String),
}

/// Extract all `{{name}}` placeholders from a command string.
/// Returns deduplicated names in the order they first appear.
pub fn extract_placeholders(command: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{'
            && chars.peek() == Some(&'{')
        {
            chars.next(); // consume second '{'
            let mut name = String::new();
            let mut closed = false;
            while let Some(nc) = chars.next() {
                if nc == '}' {
                    if chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        closed = true;
                        break;
                    } else {
                        name.push(nc);
                    }
                } else {
                    name.push(nc);
                }
            }
            if closed && !name.is_empty() && seen.insert(name.clone()) {
                result.push(name);
            }
        }
    }
    result
}

/// Replace all `{{name}}` placeholders in a command string with values from the map.
/// Returns an error if a placeholder has no corresponding value.
pub fn expand_placeholders(
    command: &str,
    values: &HashMap<String, String>,
) -> Result<String, CommandError> {
    let placeholders = extract_placeholders(command);
    let mut result = command.to_string();
    for name in &placeholders {
        match values.get(name) {
            Some(value) => {
                let pattern = format!("{{{{{}}}}}", name);
                result = result.replace(&pattern, value);
            }
            None => return Err(CommandError::UnresolvedPlaceholder(name.clone())),
        }
    }
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run `ssh <host_alias> <command>` and capture stdout/stderr.
pub fn run_and_capture(host_alias: &str, command: &str) -> Result<CapturedOutput, CommandError> {
    let output = std::process::Command::new("ssh")
        .args([host_alias, command])
        .output()?;

    Ok(CapturedOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Build args for an interactive SSH session that runs a command then drops to $SHELL.
/// Produces: `["ssh", "alias", "-t", "cmd; $SHELL"]`
pub fn build_session_args(host_alias: &str, command: &str) -> Vec<String> {
    vec![
        "ssh".to_string(),
        host_alias.to_string(),
        "-t".to_string(),
        format!("{}; $SHELL", command),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_placeholders() {
        let result = extract_placeholders("ls -la /var/log");
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_placeholder() {
        let result = extract_placeholders("tail -f {{logfile}}");
        assert_eq!(result, vec!["logfile"]);
    }

    #[test]
    fn test_multiple_placeholders() {
        let result = extract_placeholders("grep {{pattern}} {{file}}");
        assert_eq!(result, vec!["pattern", "file"]);
    }

    #[test]
    fn test_dedup_placeholders() {
        let result = extract_placeholders("echo {{name}} {{name}} {{other}}");
        assert_eq!(result, vec!["name", "other"]);
    }

    #[test]
    fn test_empty_braces_ignored() {
        // {{}} should not be treated as a valid placeholder (empty name)
        let result = extract_placeholders("echo {{}}");
        assert!(result.is_empty());
    }

    #[test]
    fn test_expand_placeholders() {
        let mut values = HashMap::new();
        values.insert("logfile".to_string(), "/var/log/app.log".to_string());
        let result = expand_placeholders("tail -f {{logfile}}", &values).unwrap();
        assert_eq!(result, "tail -f /var/log/app.log");
    }

    #[test]
    fn test_expand_missing_fails() {
        let values = HashMap::new();
        let err = expand_placeholders("tail -f {{logfile}}", &values).unwrap_err();
        assert!(matches!(err, CommandError::UnresolvedPlaceholder(name) if name == "logfile"));
    }

    #[test]
    fn test_expand_multiple_placeholders() {
        let mut values = HashMap::new();
        values.insert("pattern".to_string(), "ERROR".to_string());
        values.insert("file".to_string(), "/var/log/syslog".to_string());
        let result = expand_placeholders("grep {{pattern}} {{file}}", &values).unwrap();
        assert_eq!(result, "grep ERROR /var/log/syslog");
    }

    #[test]
    fn test_build_session_args() {
        let args = build_session_args("myserver", "htop");
        assert_eq!(
            args,
            vec!["ssh", "myserver", "-t", "htop; $SHELL"]
        );
    }
}
