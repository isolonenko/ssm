# ssm

A fast TUI for managing SSH connections, tunnels, and per-host command snippets.

## Features

- **Host management** — add, edit, delete SSH hosts with a step-by-step wizard
- **SSH config sync** — aliases work system-wide via `~/.ssh/config` Include
- **Tunnel management** — start/stop SSH tunnels with background process tracking
- **Per-host commands** — save and run diagnostic commands scoped to each host
- **Fuzzy filter** — search by alias, hostname, or tags
- **Terminal-aware** — splits tmux/zellij panes or opens tabs in your terminal

## Install

```bash
cargo install --path crates/ssm
```

## Usage

Launch the TUI:
```bash
ssm
```

Manage tunnels via CLI:
```bash
ssm tunnel start prod-api postgres
ssm tunnel status
ssm tunnel stop prod-api
```

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate |
| `/` | Filter |
| `Enter` | SSH connect |
| `T` | Tunnel menu |
| `C` | Command picker |
| `A` | Add host |
| `E` | Edit host |
| `D` | Delete host |
| `?` | Help |
| `q` | Quit |
