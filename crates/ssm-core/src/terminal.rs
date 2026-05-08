use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalContext {
    Tmux,
    Zellij,
    Iterm2,
    Kitty,
    WezTerm,
    Alacritty,
    GhosttyOrOther(String),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpawnResult {
    Spawned,
    Foreground,
}

pub fn detect_context() -> TerminalContext {
    if std::env::var("TMUX").is_ok() {
        return TerminalContext::Tmux;
    }
    if std::env::var("ZELLIJ").is_ok() {
        return TerminalContext::Zellij;
    }
    if std::env::var("KITTY_PID").is_ok() {
        return TerminalContext::Kitty;
    }
    if std::env::var("WEZTERM_EXECUTABLE").is_ok() {
        return TerminalContext::WezTerm;
    }
    if std::env::var("ALACRITTY_SOCKET").is_ok() {
        return TerminalContext::Alacritty;
    }
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        match term_program.to_lowercase().as_str() {
            "iterm.app" => return TerminalContext::Iterm2,
            "ghostty" => return TerminalContext::GhosttyOrOther("ghostty".to_string()),
            other if !other.is_empty() => {
                return TerminalContext::GhosttyOrOther(other.to_string())
            }
            _ => {}
        }
    }
    TerminalContext::Unknown
}

pub fn spawn_ssh_command(alias: &str, extra_args: &[String]) -> Vec<String> {
    let mut args = vec!["ssh".to_string(), alias.to_string()];
    args.extend_from_slice(extra_args);
    args
}

pub fn spawn_in_context(context: &TerminalContext, ssh_args: &[String]) -> SpawnResult {
    match context {
        TerminalContext::Tmux => {
            let ssh_cmd = ssh_args.join(" ");
            let _ = Command::new("tmux")
                .args(["split-window", "-h", &ssh_cmd])
                .status();
            SpawnResult::Spawned
        }
        TerminalContext::Zellij => {
            let mut args = vec!["run".to_string(), "--".to_string()];
            args.extend_from_slice(ssh_args);
            let _ = Command::new("zellij").args(&args).status();
            SpawnResult::Spawned
        }
        TerminalContext::Iterm2 => {
            let ssh_cmd = ssh_args.join(" ");
            let script = format!(
                r#"tell application "iTerm2"
  tell current window
    create tab with default profile
    tell current session
      write text "{}"
    end tell
  end tell
end tell"#,
                ssh_cmd.replace('"', "\\\"")
            );
            let _ = Command::new("osascript").arg("-e").arg(&script).status();
            SpawnResult::Spawned
        }
        TerminalContext::Kitty => {
            let mut args = vec!["launch".to_string(), "--type=tab".to_string()];
            args.extend_from_slice(ssh_args);
            let _ = Command::new("kitten").args(&args).status();
            SpawnResult::Spawned
        }
        TerminalContext::WezTerm => {
            let mut args = vec!["cli".to_string(), "spawn".to_string(), "--".to_string()];
            args.extend_from_slice(ssh_args);
            let _ = Command::new("wezterm").args(&args).status();
            SpawnResult::Spawned
        }
        TerminalContext::Alacritty => {
            let mut args = vec!["msg".to_string(), "create-window".to_string(), "--".to_string()];
            args.extend_from_slice(ssh_args);
            let _ = Command::new("alacritty").args(&args).status();
            SpawnResult::Spawned
        }
        TerminalContext::GhosttyOrOther(_) | TerminalContext::Unknown => {
            spawn_foreground(ssh_args);
            SpawnResult::Foreground
        }
    }
}

pub fn spawn_foreground(ssh_args: &[String]) {
    if ssh_args.is_empty() {
        return;
    }
    let program = &ssh_args[0];
    let args = &ssh_args[1..];
    let _ = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_ssh_command_basic() {
        let args = spawn_ssh_command("myserver", &[]);
        assert_eq!(args, vec!["ssh", "myserver"]);
    }

    #[test]
    fn test_spawn_ssh_command_extra_args() {
        let extra = vec!["-v".to_string(), "-p".to_string(), "2222".to_string()];
        let args = spawn_ssh_command("myserver", &extra);
        assert_eq!(args, vec!["ssh", "myserver", "-v", "-p", "2222"]);
    }

    #[test]
    fn test_detect_context_does_not_panic() {
        // Just ensure it runs without panicking regardless of environment
        let _ = detect_context();
    }
}
