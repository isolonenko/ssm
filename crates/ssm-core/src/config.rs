use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub ssh_config_path: PathBuf,
    pub generated_config_path: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        let ssh_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".ssh");
        Self {
            ssh_config_path: ssh_dir.join("config"),
            generated_config_path: ssh_dir.join("ssm-hosts.conf"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TunnelConfig {
    pub name: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandConfig {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Host {
    pub alias: String,
    pub hostname: String,
    pub user: Option<String>,
    #[serde(default = "default_port")]
    pub port: u16,
    pub identity_file: Option<PathBuf>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub tunnels: Vec<TunnelConfig>,
    #[serde(default)]
    pub commands: Vec<CommandConfig>,
}

fn default_port() -> u16 {
    22
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenarioTunnel {
    pub host: String,
    pub tunnel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub tunnels: Vec<ScenarioTunnel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub hosts: Vec<Host>,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            hosts: Vec::new(),
            scenarios: Vec::new(),
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf, ConfigError> {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("ssm");
        Ok(dir)
    }

    pub fn default_path() -> Result<PathBuf, ConfigError> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load(path: &std::path::Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_has_empty_hosts() {
        let config = Config::default();
        assert!(config.hosts.is_empty());
    }

    #[test]
    fn test_roundtrip_empty_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config::default();
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.hosts.len(), 0);
    }

    #[test]
    fn test_roundtrip_with_hosts() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config {
            settings: Settings::default(),
            hosts: vec![Host {
                alias: "prod-api".into(),
                hostname: "10.0.1.50".into(),
                user: Some("deploy".into()),
                port: 22,
                identity_file: Some(PathBuf::from("~/.ssh/id_ed25519")),
                tags: vec!["prod".into(), "api".into()],
                notes: Some("Main API server".into()),
                tunnels: vec![TunnelConfig {
                    name: "postgres".into(),
                    local_port: 5432,
                    remote_host: "localhost".into(),
                    remote_port: 5432,
                }],
                commands: vec![CommandConfig {
                    name: "logs".into(),
                    command: "tail -f /var/log/app/api.log".into(),
                }],
            }],
            scenarios: vec![],
        };
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.toml");
        let config = Config::load(&path).unwrap();
        assert!(config.hosts.is_empty());
    }

    #[test]
    fn test_default_port_is_22() {
        let toml_str = r#"
[[hosts]]
alias = "test"
hostname = "1.2.3.4"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hosts[0].port, 22);
    }
}
