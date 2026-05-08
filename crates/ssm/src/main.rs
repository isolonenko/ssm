mod cli;
mod tui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ssm", about = "SSH connection, tunnel, and command manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage SSH tunnels
    Tunnel {
        #[command(subcommand)]
        action: cli::tunnel::TunnelAction,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Tunnel { action }) => cli::tunnel::handle(action),
        None => tui::run(),
    }
}
