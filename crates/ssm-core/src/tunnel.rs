use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

use crate::config::TunnelConfig;

#[derive(Error, Debug)]
pub enum TunnelError {
    #[error("host '{0}' not found")]
    HostNotFound(String),
    #[error("tunnel '{0}' not found")]
    TunnelNotFound(String),
    #[error("tunnel '{0}' is already running (pid {1})")]
    AlreadyRunning(String, u32),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TunnelEntry {
    pub host_alias: String,
    pub tunnel_name: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelRegistry {
    pub entries: Vec<TunnelEntry>,
}

impl TunnelRegistry {
    pub fn load(path: &std::path::Path) -> Result<Self, TunnelError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let registry: TunnelRegistry = serde_json::from_str(&content)?;
        Ok(registry)
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), TunnelError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Remove entries whose PIDs are no longer alive.
    pub fn reconcile(&mut self) {
        self.entries.retain(|e| is_pid_alive(e.pid));
    }

    pub fn find(&self, host_alias: &str, tunnel_name: &str) -> Option<&TunnelEntry> {
        self.entries
            .iter()
            .find(|e| e.host_alias == host_alias && e.tunnel_name == tunnel_name)
    }

    pub fn for_host(&self, host_alias: &str) -> Vec<&TunnelEntry> {
        self.entries
            .iter()
            .filter(|e| e.host_alias == host_alias)
            .collect()
    }

    pub fn add(&mut self, entry: TunnelEntry) {
        self.entries.push(entry);
    }

    pub fn remove(&mut self, host_alias: &str, tunnel_name: &str) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|e| !(e.host_alias == host_alias && e.tunnel_name == tunnel_name));
        self.entries.len() < before
    }
}

pub fn is_pid_alive(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

pub fn registry_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("ssm")
        .join("tunnels.json")
}

pub fn start_tunnel(
    host_alias: &str,
    tunnel: &TunnelConfig,
    registry: &mut TunnelRegistry,
) -> Result<TunnelEntry, TunnelError> {
    // Check if already running
    if let Some(existing) = registry.find(host_alias, &tunnel.name) {
        if is_pid_alive(existing.pid) {
            return Err(TunnelError::AlreadyRunning(
                tunnel.name.clone(),
                existing.pid,
            ));
        }
        // Dead entry — remove it before starting a new one
        registry.remove(host_alias, &tunnel.name);
    }

    let forward_spec = format!(
        "{}:{}:{}",
        tunnel.local_port, tunnel.remote_host, tunnel.remote_port
    );

    let child = std::process::Command::new("ssh")
        .args([
            "-N",
            "-L",
            &forward_spec,
            "-o",
            "ExitOnForwardFailure=yes",
            host_alias,
        ])
        .spawn()?;

    let pid = child.id();
    // Prevent the Child destructor from running: on Unix, dropping a Child
    // does not kill the process but leaves an un-waited zombie when it exits.
    // forget() hands ownership to the OS so we manage the lifecycle via PID only.
    std::mem::forget(child);
    let entry = TunnelEntry {
        host_alias: host_alias.to_string(),
        tunnel_name: tunnel.name.clone(),
        local_port: tunnel.local_port,
        remote_host: tunnel.remote_host.clone(),
        remote_port: tunnel.remote_port,
        pid,
    };
    registry.add(entry.clone());
    Ok(entry)
}

pub fn stop_tunnel(
    host_alias: &str,
    tunnel_name: &str,
    registry: &mut TunnelRegistry,
) -> Result<(), TunnelError> {
    let entry = registry
        .find(host_alias, tunnel_name)
        .ok_or_else(|| TunnelError::TunnelNotFound(tunnel_name.to_string()))?
        .clone();

    // Send SIGTERM
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let _ = kill(Pid::from_raw(entry.pid as i32), Signal::SIGTERM);

    registry.remove(host_alias, tunnel_name);
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelStatus {
    Running { pid: u32 },
    Stopped,
}

pub fn check_tunnel_status(
    host_alias: &str,
    tunnel_name: &str,
    registry: &TunnelRegistry,
) -> TunnelStatus {
    match registry.find(host_alias, tunnel_name) {
        Some(entry) if is_pid_alive(entry.pid) => TunnelStatus::Running { pid: entry.pid },
        _ => TunnelStatus::Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(host: &str, name: &str, pid: u32) -> TunnelEntry {
        TunnelEntry {
            host_alias: host.to_string(),
            tunnel_name: name.to_string(),
            local_port: 5432,
            remote_host: "localhost".to_string(),
            remote_port: 5432,
            pid,
        }
    }

    #[test]
    fn test_registry_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tunnels.json");

        let mut registry = TunnelRegistry::default();
        registry.add(make_entry("prod", "postgres", 12345));

        registry.save(&path).unwrap();
        let loaded = TunnelRegistry::load(&path).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].tunnel_name, "postgres");
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.json");
        let registry = TunnelRegistry::load(&path).unwrap();
        assert!(registry.entries.is_empty());
    }

    #[test]
    fn test_find() {
        let mut registry = TunnelRegistry::default();
        registry.add(make_entry("prod", "postgres", 111));
        registry.add(make_entry("staging", "redis", 222));

        assert!(registry.find("prod", "postgres").is_some());
        assert!(registry.find("prod", "redis").is_none());
        assert!(registry.find("staging", "redis").is_some());
    }

    #[test]
    fn test_for_host() {
        let mut registry = TunnelRegistry::default();
        registry.add(make_entry("prod", "postgres", 111));
        registry.add(make_entry("prod", "redis", 222));
        registry.add(make_entry("staging", "postgres", 333));

        let prod_entries = registry.for_host("prod");
        assert_eq!(prod_entries.len(), 2);
        let staging_entries = registry.for_host("staging");
        assert_eq!(staging_entries.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut registry = TunnelRegistry::default();
        registry.add(make_entry("prod", "postgres", 111));
        registry.add(make_entry("prod", "redis", 222));

        let removed = registry.remove("prod", "postgres");
        assert!(removed);
        assert_eq!(registry.entries.len(), 1);
        assert_eq!(registry.entries[0].tunnel_name, "redis");

        // Removing non-existent returns false
        let not_removed = registry.remove("prod", "postgres");
        assert!(!not_removed);
    }

    #[test]
    fn test_reconcile_dead_pids() {
        let mut registry = TunnelRegistry::default();
        // Use PID 999999 which is almost certainly not running
        registry.add(make_entry("prod", "postgres", 999_999_u32));

        // Use the current process PID which is definitely alive
        let live_pid = std::process::id();
        registry.add(make_entry("prod", "redis", live_pid));

        registry.reconcile();

        // After reconcile, dead PID entry should be removed; live PID should remain
        let postgres_entry = registry.find("prod", "postgres");
        let redis_entry = registry.find("prod", "redis");

        // postgres (dead pid) should be removed
        assert!(postgres_entry.is_none());
        // redis (live pid) should still be there
        assert!(redis_entry.is_some());
    }

    #[test]
    fn test_check_status_stopped() {
        let registry = TunnelRegistry::default();
        let status = check_tunnel_status("prod", "postgres", &registry);
        assert_eq!(status, TunnelStatus::Stopped);
    }
}
