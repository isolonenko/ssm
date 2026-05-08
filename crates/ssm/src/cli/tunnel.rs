use clap::Subcommand;
use ssm_core::config::Config;
use ssm_core::tunnel::{is_pid_alive, registry_path, start_tunnel, stop_tunnel, TunnelRegistry};

#[derive(Subcommand)]
pub enum TunnelAction {
    /// Start a tunnel
    Start {
        alias: String,
        name: Option<String>,
    },
    /// Stop a tunnel
    Stop {
        alias: String,
        name: Option<String>,
    },
    /// Show tunnel status
    Status,
}

pub fn handle(action: TunnelAction) {
    let config_path = Config::default_path().expect("failed to determine config path");
    let config = Config::load(&config_path).expect("failed to load config");
    let reg_path = registry_path();
    let mut registry = TunnelRegistry::load(&reg_path).expect("failed to load tunnel registry");
    registry.reconcile();

    match action {
        TunnelAction::Start { alias, name } => {
            let host = config
                .find_host(&alias)
                .unwrap_or_else(|| {
                    eprintln!("Host '{}' not found", alias);
                    std::process::exit(1);
                });

            let tunnels_to_start: Vec<_> = match &name {
                Some(n) => host.tunnels.iter().filter(|t| t.name == *n).collect(),
                None => host.tunnels.iter().collect(),
            };

            if tunnels_to_start.is_empty() {
                eprintln!("No tunnels found for host '{}'", alias);
                std::process::exit(1);
            }

            for tunnel in tunnels_to_start {
                match start_tunnel(&alias, tunnel, &mut registry) {
                    Ok(entry) => println!(
                        "Started tunnel {}:{} (localhost:{} → {}:{}) [PID {}]",
                        alias,
                        tunnel.name,
                        tunnel.local_port,
                        tunnel.remote_host,
                        tunnel.remote_port,
                        entry.pid
                    ),
                    Err(e) => eprintln!("Failed to start {}:{}: {}", alias, tunnel.name, e),
                }
            }
            registry.save(&reg_path).expect("failed to save registry");
        }
        TunnelAction::Stop { alias, name } => {
            match &name {
                Some(n) => {
                    stop_tunnel(&alias, n, &mut registry).expect("failed to stop tunnel");
                    println!("Stopped tunnel {}:{}", alias, n);
                }
                None => {
                    let names: Vec<String> = registry
                        .for_host(&alias)
                        .iter()
                        .map(|t| t.tunnel_name.clone())
                        .collect();
                    for n in names {
                        stop_tunnel(&alias, &n, &mut registry).expect("failed to stop tunnel");
                        println!("Stopped tunnel {}:{}", alias, n);
                    }
                }
            }
            registry.save(&reg_path).expect("failed to save registry");
        }
        TunnelAction::Status => {
            if registry.entries.is_empty() {
                println!("No tunnels running.");
                return;
            }
            println!(
                "{:<15} {:<15} {:<25} {:<8} {}",
                "HOST", "TUNNEL", "FORWARDING", "PID", "STATUS"
            );
            for entry in &registry.entries {
                let status = if is_pid_alive(entry.pid) {
                    "running"
                } else {
                    "dead"
                };
                println!(
                    "{:<15} {:<15} localhost:{} → {}:{:<5} {:<8} {}",
                    entry.host_alias,
                    entry.tunnel_name,
                    entry.local_port,
                    entry.remote_host,
                    entry.remote_port,
                    entry.pid,
                    status
                );
            }
        }
    }
}
