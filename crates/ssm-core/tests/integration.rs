use ssm_core::config::{CommandConfig, Config, Host, Settings, TunnelConfig};
use ssm_core::ssh_config;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_full_workflow_add_hosts_and_sync() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let ssh_config_path = dir.path().join("ssh_config");
    let generated_path = dir.path().join("ssm-hosts.conf");

    let mut config = Config {
        settings: Settings {
            ssh_config_path: ssh_config_path.clone(),
            generated_config_path: generated_path.clone(),
        },
        hosts: vec![],
    };

    config
        .add_host(Host {
            alias: "prod-api".into(),
            hostname: "10.0.1.50".into(),
            user: Some("deploy".into()),
            port: 22,
            identity_file: Some(PathBuf::from("~/.ssh/id_ed25519")),
            tags: vec!["prod".into(), "api".into()],
            notes: Some("Main API".into()),
            tunnels: vec![TunnelConfig {
                name: "postgres".into(),
                local_port: 5432,
                remote_host: "localhost".into(),
                remote_port: 5432,
            }],
            commands: vec![CommandConfig {
                name: "logs".into(),
                command: "tail -f /var/log/app.log".into(),
            }],
        })
        .unwrap();

    config
        .add_host(Host {
            alias: "staging".into(),
            hostname: "10.0.2.50".into(),
            user: Some("deploy".into()),
            port: 2222,
            identity_file: None,
            tags: vec!["staging".into()],
            notes: None,
            tunnels: vec![],
            commands: vec![],
        })
        .unwrap();

    config.save(&config_path).unwrap();
    ssh_config::sync_ssh_config(&config).unwrap();

    let generated = std::fs::read_to_string(&generated_path).unwrap();
    assert!(generated.contains("Host prod-api"));
    assert!(generated.contains("HostName 10.0.1.50"));
    assert!(generated.contains("User deploy"));
    assert!(generated.contains("Host staging"));
    assert!(generated.contains("Port 2222"));

    let ssh_cfg = std::fs::read_to_string(&ssh_config_path).unwrap();
    assert!(ssh_cfg.contains("Include ssm-hosts.conf"));

    let reloaded = Config::load(&config_path).unwrap();
    assert_eq!(reloaded.hosts.len(), 2);
    assert_eq!(reloaded.hosts[0].tunnels.len(), 1);
    assert_eq!(reloaded.hosts[0].commands.len(), 1);

    config.remove_host("staging").unwrap();
    config.save(&config_path).unwrap();
    ssh_config::sync_ssh_config(&config).unwrap();

    let generated = std::fs::read_to_string(&generated_path).unwrap();
    assert!(generated.contains("Host prod-api"));
    assert!(!generated.contains("Host staging"));
}
