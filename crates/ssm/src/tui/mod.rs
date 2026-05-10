pub mod app;
mod command_picker;
mod confirm;
mod detail_panel;
mod filter;
mod help;
mod host_list;
mod import;
mod output_viewer;
mod scenario;
mod tunnel_menu;
mod tunnel_wizard;
mod wizard;

use app::{App, ConfirmAction, Mode, TunnelWizardState, TunnelWizardStep, WizardState, WizardStep};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use ssm_core::{
    command::{build_session_args, run_and_capture},
    config::{Config, Host},
    terminal::{detect_context, spawn_foreground, spawn_in_context, spawn_ssh_command},
    tunnel::{is_pid_alive, registry_path, start_tunnel, stop_tunnel, TunnelRegistry},
};
use std::{io, path::PathBuf};

pub fn run() {
    let config_path = Config::default_path().expect("failed to determine config path");
    let mut config = Config::load(&config_path).expect("failed to load config");

    // First run: import hosts from ~/.ssh/config
    if config.hosts.is_empty() {
        let ssh_config_path = &config.settings.ssh_config_path;
        let imported = ssm_core::import::parse_ssh_config(ssh_config_path);
        if !imported.is_empty() {
            config.hosts = imported;
            let _ = config.save(&config_path);
            let _ = ssm_core::ssh_config::sync_ssh_config(&config);
        }
    }

    let reg_path = registry_path();
    let mut registry = TunnelRegistry::load(&reg_path).expect("failed to load tunnel registry");
    registry.reconcile();

    let mut app = App::new(config, config_path, registry, reg_path);

    // Set up terminal
    enable_raw_mode().expect("failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableBracketedPaste
    )
    .expect("failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    // Event loop
    loop {
        terminal
            .draw(|f| draw(f, &app))
            .expect("failed to draw frame");

        if event::poll(std::time::Duration::from_millis(250)).expect("event poll failed") {
            match event::read().expect("event read failed") {
                Event::Key(key) => {
                    handle_input(&mut app, key.code, key.modifiers);
                }
                Event::Paste(text) => {
                    if matches!(app.mode, Mode::ImportPaste) {
                        app.import_buffer.push_str(&text);
                    }
                }
                _ => {}
            }
        }

        // Task 15: pending foreground SSH — suspend TUI, run SSH, restore TUI
        if let Some(ssh_args) = app.pending_ssh.take() {
            disable_raw_mode().expect("failed to disable raw mode");
            execute!(terminal.backend_mut(), LeaveAlternateScreen)
                .expect("failed to leave alternate screen");
            terminal.show_cursor().expect("failed to show cursor");

            spawn_foreground(&ssh_args);

            enable_raw_mode().expect("failed to enable raw mode");
            execute!(terminal.backend_mut(), EnterAlternateScreen)
                .expect("failed to enter alternate screen");
            terminal.clear().expect("failed to clear terminal");
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode().expect("failed to disable raw mode");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableBracketedPaste
    )
    .expect("failed to leave alternate screen");
    terminal.show_cursor().expect("failed to show cursor");
}

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Dark background fill
    let bg = Block::default().style(Style::default().bg(Color::Rgb(15, 15, 20)));
    f.render_widget(bg, area);

    let outer_block = Block::default()
        .title(" ssm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)));
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    if app.config.hosts.is_empty() && matches!(app.mode, Mode::Normal | Mode::Filter) {
        // Empty state — center the message
        // Layout: [content, filter bar, status bar]
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner_area);

        let msg_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(3),
                Constraint::Percentage(52),
            ])
            .split(v_chunks[0]);

        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "No connections yet. Press A to add your first host.",
            Style::default().add_modifier(Modifier::DIM),
        )]))
        .alignment(Alignment::Center);
        f.render_widget(msg, msg_chunks[1]);

        // Filter bar
        let is_filter = matches!(app.mode, Mode::Filter);
        filter::render(f, v_chunks[1], &app.filter_text, is_filter);

        // Status bar
        render_status_bar(f, v_chunks[2], app);
    } else {
        // Main layout: vertical [content, filter, status]
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner_area);

        // Content area: horizontal [host list 35%, detail panel 65%]
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(v_chunks[0]);

        // Visible (filtered) hosts
        let visible_hosts: Vec<Host> = app
            .filtered_indices
            .iter()
            .map(|&i| app.config.hosts[i].clone())
            .collect();

        host_list::render(f, h_chunks[0], &visible_hosts, app.selected_index, &app.registry);

        // Detail panel
        if let Some(host) = app.selected_host() {
            detail_panel::render(f, h_chunks[1], host, &app.registry);
        } else {
            let placeholder = Paragraph::new("").block(
                Block::default()
                    .title(" Detail ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
                    .style(Style::default().bg(Color::Rgb(15, 15, 20))),
            );
            f.render_widget(placeholder, h_chunks[1]);
        }

        // Filter bar
        let is_filter = matches!(app.mode, Mode::Filter);
        filter::render(f, v_chunks[1], &app.filter_text, is_filter);

        // Status bar
        render_status_bar(f, v_chunks[2], app);
    }

    // Overlay modals on top
    match &app.mode {
        Mode::Wizard(state) => {
            wizard::render(f, state, "Add Host");
        }
        Mode::EditHost(state) => {
            wizard::render(f, state, "Edit Host");
        }
        Mode::Confirm(ConfirmAction::DeleteHost(alias)) => {
            confirm::render(f, &format!("Delete host '{}'?", alias));
        }
        Mode::TunnelMenu => {
            if let Some(host) = app.selected_host() {
                tunnel_menu::render(f, host, &app.registry, app.tunnel_selected);
            }
        }
        Mode::TunnelWizard(state) => {
            tunnel_wizard::render(f, state);
        }
        Mode::CommandPicker => {
            if let Some(host) = app.selected_host() {
                command_picker::render(f, &host.alias, &host.commands, app.command_selected);
            }
        }
        Mode::OutputViewer => {
            output_viewer::render(f, &app.output_text, app.output_scroll);
        }
        Mode::Help => {
            help::render(f);
        }
        Mode::ImportPaste => {
            import::render_paste(f, &app.import_buffer);
        }
        Mode::ImportPreview => {
            import::render_preview(f, &app.import_parsed, &app.config.hosts, app.import_scroll);
        }
        Mode::ScenarioMenu => {
            scenario::render_menu(f, &app.config, &app.registry, app.scenario_selected);
        }
        Mode::ScenarioCreate => {
            scenario::render_create(
                f,
                &app.scenario_name_buf,
                &app.config,
                &app.scenario_toggle,
                app.scenario_selected,
            );
        }
        _ => {}
    }
}

fn render_status_bar(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let base_hints: &str = match &app.mode {
        Mode::Normal => {
            "↑↓ navigate  / filter  Enter ssh  T tunnel  S scenario  C cmd  A add  E edit  D del  I import  ? help"
        }
        Mode::Filter => "Enter: apply  Esc: cancel",
        Mode::Wizard(_) | Mode::EditHost(_) => "Enter: next  Esc: cancel",
        Mode::Confirm(_) => "y: confirm  n/Esc: cancel",
        Mode::CommandPicker => "Enter: run & show  S: session  Y: copy  Esc: close",
        Mode::OutputViewer => "↑↓: scroll  Esc: close",
        Mode::TunnelMenu => "Enter: toggle  A: add tunnel  Esc: close",
        Mode::TunnelWizard(_) => "Enter: next  Esc: cancel",
        Mode::ImportPaste => "Enter: parse & preview  Esc: cancel",
        Mode::ImportPreview => "Y: apply  Esc: cancel  ↑↓: scroll",
        Mode::ScenarioMenu => "Enter: toggle all  A: create  D: delete  Esc: close",
        Mode::ScenarioCreate => "Space: toggle  Enter: save  Esc: cancel",
        Mode::Help => "Esc: close",
    };

    // Append active filter indicator when in Normal mode with a non-empty filter
    let hints = if matches!(app.mode, Mode::Normal) && !app.filter_text.is_empty() {
        format!("{}  [filter: {}]", base_hints, app.filter_text)
    } else {
        base_hints.to_string()
    };

    let para = Paragraph::new(Line::from(Span::styled(
        hints,
        Style::default().fg(Color::Gray),
    )));
    f.render_widget(para, area);
}

fn handle_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    match &app.mode.clone() {
        Mode::Normal => match key {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.should_quit = true
            }
            KeyCode::Char('j') | KeyCode::Down => app.move_selection_down(),
            KeyCode::Char('k') | KeyCode::Up => app.move_selection_up(),
            KeyCode::Char('?') => {
                app.mode = Mode::Help;
            }
            KeyCode::Char('/') if !modifiers.contains(KeyModifiers::SHIFT) => {
                app.mode = Mode::Filter;
            }
            // Task 15: SSH Connect
            KeyCode::Enter => {
                if let Some(host) = app.selected_host() {
                    let alias = host.alias.clone();
                    let ssh_args = spawn_ssh_command(&alias, &[]);
                    let ctx = detect_context();
                    // For contexts that can open a new pane/tab, spawn there.
                    // For unknown/ghostty contexts, fall back to foreground via
                    // the event loop so TUI can be properly suspended/restored.
                    use ssm_core::terminal::TerminalContext;
                    match &ctx {
                        TerminalContext::Unknown | TerminalContext::GhosttyOrOther(_) => {
                            app.pending_ssh = Some(ssh_args);
                        }
                        _ => {
                            spawn_in_context(&ctx, &ssh_args);
                        }
                    }
                }
            }
            KeyCode::Char('T') => {
                if let Some(host) = app.selected_host() {
                    let alias = host.alias.clone();
                    let tunnels = host.tunnels.clone();
                    for tunnel in &tunnels {
                        let running = app
                            .registry
                            .find(&alias, &tunnel.name)
                            .map(|e| is_pid_alive(e.pid))
                            .unwrap_or(false);
                        if running {
                            let _ = stop_tunnel(&alias, &tunnel.name, &mut app.registry);
                        } else {
                            let _ = start_tunnel(&alias, tunnel, &mut app.registry);
                        }
                    }
                    app.save_registry();
                }
            }
            KeyCode::Char('t') => {
                if let Some(host) = app.selected_host() {
                    if host.tunnels.is_empty() {
                        app.mode = Mode::TunnelWizard(TunnelWizardState::new());
                    } else {
                        app.tunnel_selected = 0;
                        app.mode = Mode::TunnelMenu;
                    }
                }
            }
            KeyCode::Char('c') => {
                if let Some(host) = app.selected_host() {
                    if !host.commands.is_empty() {
                        app.command_selected = 0;
                        app.mode = Mode::CommandPicker;
                    }
                }
            }
            KeyCode::Char('a') => {
                app.mode = Mode::Wizard(WizardState::new());
            }
            KeyCode::Char('e') => {
                if let Some(host) = app.selected_host() {
                    let state = WizardState::from_host(host);
                    app.mode = Mode::EditHost(state);
                }
            }
            KeyCode::Char('d') => {
                if let Some(alias) = app.selected_host_alias() {
                    let alias = alias.to_string();
                    app.mode = Mode::Confirm(ConfirmAction::DeleteHost(alias));
                }
            }
            KeyCode::Char('i') => {
                app.import_buffer.clear();
                app.import_parsed.clear();
                app.import_scroll = 0;
                app.mode = Mode::ImportPaste;
            }
            KeyCode::Char('s') => {
                app.scenario_selected = 0;
                app.mode = Mode::ScenarioMenu;
            }
            _ => {}
        },
        Mode::Filter => match key {
            KeyCode::Esc => {
                app.filter_text.clear();
                app.update_filter();
                app.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                app.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                app.filter_text.pop();
                app.update_filter();
            }
            KeyCode::Char(c) => {
                app.filter_text.push(c);
                app.update_filter();
            }
            _ => {}
        },
        Mode::Help => match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                app.mode = Mode::Normal;
            }
            _ => {}
        },
        // Task 16: Tunnel menu navigation
        Mode::TunnelMenu => {
            // Defensive: if there is no selected host, exit the menu immediately.
            if app.selected_host().is_none() {
                app.mode = Mode::Normal;
                return;
            }
            let host = app.selected_host().cloned().unwrap();
            match key {
                KeyCode::Esc => {
                    app.mode = Mode::Normal;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let max = host.tunnels.len().saturating_sub(1);
                    if app.tunnel_selected < max {
                        app.tunnel_selected += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if app.tunnel_selected > 0 {
                        app.tunnel_selected -= 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(tunnel) = host.tunnels.get(app.tunnel_selected) {
                        let tunnel = tunnel.clone();
                        let alias = host.alias.clone();
                        let running = app
                            .registry
                            .find(&alias, &tunnel.name)
                            .map(|e| is_pid_alive(e.pid))
                            .unwrap_or(false);
                        if running {
                            let _ = stop_tunnel(&alias, &tunnel.name, &mut app.registry);
                        } else {
                            let _ = start_tunnel(&alias, &tunnel, &mut app.registry);
                        }
                        app.save_registry();
                    }
                }
                KeyCode::Char('a') => {
                    app.mode = Mode::TunnelWizard(TunnelWizardState::new());
                }
                _ => {}
            }
        }
        // Task 17: Command picker
        Mode::CommandPicker => {
            if let Some(host) = app.selected_host().cloned() {
                match key {
                    KeyCode::Esc => {
                        app.mode = Mode::Normal;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = host.commands.len().saturating_sub(1);
                        if app.command_selected < max {
                            app.command_selected += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.command_selected > 0 {
                            app.command_selected -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Run & show: capture output, switch to OutputViewer
                        if let Some(cmd) = host.commands.get(app.command_selected) {
                            let command_str = cmd.command.clone();
                            let alias = host.alias.clone();
                            match run_and_capture(&alias, &command_str) {
                                Ok(captured) => {
                                    let mut output = captured.stdout;
                                    if !captured.stderr.is_empty() {
                                        if !output.is_empty() {
                                            output.push('\n');
                                        }
                                        output.push_str("--- stderr ---\n");
                                        output.push_str(&captured.stderr);
                                    }
                                    app.output_text = output;
                                }
                                Err(e) => {
                                    app.output_text = format!("Error: {}", e);
                                }
                            }
                            app.output_scroll = 0;
                            app.mode = Mode::OutputViewer;
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        // Run in session
                        if let Some(cmd) = host.commands.get(app.command_selected) {
                            let session_args =
                                build_session_args(&host.alias, &cmd.command);
                            let ctx = detect_context();
                            use ssm_core::terminal::TerminalContext;
                            match &ctx {
                                TerminalContext::Unknown | TerminalContext::GhosttyOrOther(_) => {
                                    app.pending_ssh = Some(session_args);
                                }
                                _ => {
                                    spawn_in_context(&ctx, &session_args);
                                }
                            }
                        }
                        app.mode = Mode::Normal;
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        // Copy command to clipboard
                        if let Some(cmd) = host.commands.get(app.command_selected) {
                            let command_str = cmd.command.clone();
                            let _ = try_copy_to_clipboard(&command_str);
                        }
                        app.mode = Mode::Normal;
                    }
                    _ => {}
                }
            } else {
                if key == KeyCode::Esc {
                    app.mode = Mode::Normal;
                }
            }
        }
        // Task 17: Output viewer
        Mode::OutputViewer => match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.mode = Mode::Normal;
                app.output_text.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.output_scroll = app.output_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.output_scroll = app.output_scroll.saturating_sub(1);
            }
            _ => {}
        },
        Mode::ImportPaste => match key {
            KeyCode::Esc => {
                app.import_buffer.clear();
                app.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let parsed = ssm_core::import::parse_ssh_commands(&app.import_buffer);
                app.import_parsed = parsed;
                app.import_scroll = 0;
                app.mode = Mode::ImportPreview;
            }
            KeyCode::Backspace => {
                app.import_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.import_buffer.push(c);
            }
            _ => {}
        },
        Mode::ImportPreview => match key {
            KeyCode::Esc => {
                app.mode = Mode::ImportPaste;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.import_scroll = app.import_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.import_scroll = app.import_scroll.saturating_sub(1);
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                apply_import(app);
                app.mode = Mode::Normal;
            }
            _ => {}
        },
        Mode::ScenarioMenu => match key {
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = app.config.scenarios.len().saturating_sub(1);
                if app.scenario_selected < max {
                    app.scenario_selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.scenario_selected > 0 {
                    app.scenario_selected -= 1;
                }
            }
            KeyCode::Enter => {
                if let Some(scenario) = app.config.scenarios.get(app.scenario_selected).cloned() {
                    toggle_scenario(app, &scenario);
                }
            }
            KeyCode::Char('a') => {
                let all_tunnels = scenario::collect_all_tunnels(&app.config);
                app.scenario_name_buf.clear();
                app.scenario_toggle = vec![false; all_tunnels.len()];
                app.scenario_selected = 0;
                app.mode = Mode::ScenarioCreate;
            }
            KeyCode::Char('d') => {
                if !app.config.scenarios.is_empty() {
                    app.config.scenarios.remove(app.scenario_selected);
                    app.save_config();
                    if app.scenario_selected > 0
                        && app.scenario_selected >= app.config.scenarios.len()
                    {
                        app.scenario_selected = app.config.scenarios.len().saturating_sub(1);
                    }
                }
            }
            _ => {}
        },
        Mode::ScenarioCreate => match key {
            KeyCode::Esc => {
                app.mode = Mode::ScenarioMenu;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let all_tunnels = scenario::collect_all_tunnels(&app.config);
                let max = all_tunnels.len().saturating_sub(1);
                if app.scenario_selected < max {
                    app.scenario_selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.scenario_selected > 0 {
                    app.scenario_selected -= 1;
                }
            }
            KeyCode::Char(' ') => {
                if let Some(val) = app.scenario_toggle.get_mut(app.scenario_selected) {
                    *val = !*val;
                }
            }
            KeyCode::Enter => {
                save_scenario(app);
                app.mode = Mode::ScenarioMenu;
            }
            KeyCode::Backspace => {
                app.scenario_name_buf.pop();
            }
            KeyCode::Char(c) => {
                // If cursor is still on name (selected == 0 and name is being typed)
                // We'll use a simple heuristic: if no toggles are set and we're at position 0,
                // typing goes to name. Once you press down, navigation takes over.
                // Actually, let's just always allow typing into name while in this mode.
                // Characters that aren't j/k/space go to name buffer.
                app.scenario_name_buf.push(c);
            }
            _ => {}
        },
        Mode::TunnelWizard(state) => {
            handle_tunnel_wizard_input(app, key, state.clone());
        }
        Mode::Wizard(state) => {
            handle_wizard_input(app, key, state.clone(), false);
        }
        Mode::EditHost(state) => {
            handle_wizard_input(app, key, state.clone(), true);
        }
        Mode::Confirm(ConfirmAction::DeleteHost(alias)) => {
            let alias = alias.clone();
            match key {
                KeyCode::Char('y') => {
                    let _ = app.config.remove_host(&alias);
                    app.save_config();
                    let _ = ssm_core::ssh_config::sync_ssh_config(&app.config);
                    app.update_filter();
                    app.mode = Mode::Normal;
                }
                KeyCode::Esc | KeyCode::Char('n') => {
                    app.mode = Mode::Normal;
                }
                _ => {}
            }
        }
    }
}

/// Attempt to copy a string to the system clipboard via arboard.
/// Failures are silently ignored.
fn try_copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text.to_owned());
    }
}

/// Shared input handler for Wizard (add) and EditHost modes.
/// `is_edit` = true means we call update_host instead of add_host.
fn handle_wizard_input(app: &mut App, key: KeyCode, mut state: WizardState, is_edit: bool) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if state.step == WizardStep::Tags {
                // Final step — commit
                let host = Host {
                    alias: state.alias.clone(),
                    hostname: state.hostname.clone(),
                    user: if state.user.is_empty() {
                        None
                    } else {
                        Some(state.user.clone())
                    },
                    port: state.port.parse().unwrap_or(22),
                    identity_file: if state.identity_file.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(&state.identity_file))
                    },
                    tags: state
                        .tags
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    notes: None,
                    // Preserve existing tunnels/commands when editing; empty for new hosts.
                    tunnels: state.original_tunnels.clone(),
                    commands: state.original_commands.clone(),
                };

                if is_edit {
                    // We need the original alias. For edit mode we started from
                    // WizardState::from_host which sets alias = original alias.
                    // The original alias is stored in state.alias before any edits —
                    // but since Enter on the final step means the user has already
                    // navigated through all steps (potentially changing alias), we
                    // need to retrieve the original alias from the app's selected host.
                    let original_alias = app
                        .selected_host()
                        .map(|h| h.alias.clone())
                        .unwrap_or_else(|| state.alias.clone());
                    let _ = app.config.update_host(&original_alias, host);
                } else {
                    let _ = app.config.add_host(host);
                }

                app.save_config();
                let _ = ssm_core::ssh_config::sync_ssh_config(&app.config);
                app.update_filter();
                app.mode = Mode::Normal;
            } else {
                state.next_step();
                if is_edit {
                    app.mode = Mode::EditHost(state);
                } else {
                    app.mode = Mode::Wizard(state);
                }
            }
        }
        KeyCode::Backspace => {
            let val = state.current_value_mut();
            if val.is_empty() {
                state.prev_step();
            } else {
                val.pop();
            }
            if is_edit {
                app.mode = Mode::EditHost(state);
            } else {
                app.mode = Mode::Wizard(state);
            }
        }
        KeyCode::Char(c) => {
            state.current_value_mut().push(c);
            if is_edit {
                app.mode = Mode::EditHost(state);
            } else {
                app.mode = Mode::Wizard(state);
            }
        }
        _ => {}
    }
}

fn handle_tunnel_wizard_input(app: &mut App, key: KeyCode, mut state: TunnelWizardState) {
    use ssm_core::config::TunnelConfig;

    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if state.step == TunnelWizardStep::RemotePort {
                let local_port = match state.local_port.parse::<u16>() {
                    Ok(p) => p,
                    Err(_) => {
                        app.status_message = Some("Invalid local port".to_string());
                        app.mode = Mode::Normal;
                        return;
                    }
                };
                let remote_port = match state.remote_port.parse::<u16>() {
                    Ok(p) => p,
                    Err(_) => {
                        app.status_message = Some("Invalid remote port".to_string());
                        app.mode = Mode::Normal;
                        return;
                    }
                };
                if state.name.trim().is_empty() {
                    app.status_message = Some("Tunnel name is required".to_string());
                    app.mode = Mode::Normal;
                    return;
                }

                let tunnel = TunnelConfig {
                    name: state.name.trim().to_string(),
                    local_port,
                    remote_host: if state.remote_host.trim().is_empty() {
                        "localhost".to_string()
                    } else {
                        state.remote_host.trim().to_string()
                    },
                    remote_port,
                };

                if let Some(idx) = app.filtered_indices.get(app.selected_index).copied() {
                    if let Some(host) = app.config.hosts.get_mut(idx) {
                        host.tunnels.push(tunnel);
                    }
                }

                app.save_config();
                let _ = ssm_core::ssh_config::sync_ssh_config(&app.config);
                app.mode = Mode::Normal;
            } else {
                state.next_step();
                app.mode = Mode::TunnelWizard(state);
            }
        }
        KeyCode::Backspace => {
            let val = state.current_value_mut();
            if val.is_empty() {
                state.prev_step();
            } else {
                val.pop();
            }
            app.mode = Mode::TunnelWizard(state);
        }
        KeyCode::Char(c) => {
            state.current_value_mut().push(c);
            app.mode = Mode::TunnelWizard(state);
        }
        _ => {}
    }
}

fn apply_import(app: &mut App) {
    use ssm_core::config::{Host, TunnelConfig};
    use ssm_core::import::alias_from_hostname;

    let parsed = std::mem::take(&mut app.import_parsed);
    let mut count = 0;

    for parsed_host in parsed {
        let tunnels: Vec<TunnelConfig> = parsed_host
            .tunnels
            .iter()
            .map(|t| TunnelConfig {
                name: format!("port-{}", t.local_port),
                local_port: t.local_port,
                remote_host: t.remote_host.clone(),
                remote_port: t.remote_port,
            })
            .collect();

        if let Some(existing) = app
            .config
            .hosts
            .iter_mut()
            .find(|h| h.hostname == parsed_host.hostname)
        {
            for tunnel in &tunnels {
                if !existing.tunnels.iter().any(|t| t.local_port == tunnel.local_port) {
                    existing.tunnels.push(tunnel.clone());
                    count += 1;
                }
            }
        } else {
            let alias = alias_from_hostname(&parsed_host.hostname);
            // Ensure unique alias
            let final_alias = if app.config.hosts.iter().any(|h| h.alias == alias) {
                format!("{}-{}", alias, parsed_host.port)
            } else {
                alias
            };

            count += tunnels.len();
            app.config.hosts.push(Host {
                alias: final_alias,
                hostname: parsed_host.hostname,
                user: parsed_host.user,
                port: parsed_host.port,
                identity_file: None,
                tags: vec![],
                notes: None,
                tunnels,
                commands: vec![],
            });
        }
    }

    app.save_config();
    let _ = ssm_core::ssh_config::sync_ssh_config(&app.config);
    app.update_filter();
    app.import_buffer.clear();
    app.status_message = Some(format!("Imported {} tunnel(s)", count));
}

fn toggle_scenario(app: &mut App, scenario: &ssm_core::config::Scenario) {
    // If any tunnel in the scenario is running, stop all. Otherwise start all.
    let any_running = scenario.tunnels.iter().any(|st| {
        app.registry
            .find(&st.host, &st.tunnel)
            .map(|e| is_pid_alive(e.pid))
            .unwrap_or(false)
    });

    for st in &scenario.tunnels {
        if any_running {
            let _ = stop_tunnel(&st.host, &st.tunnel, &mut app.registry);
        } else {
            // Find the tunnel config from the host
            if let Some(host) = app.config.hosts.iter().find(|h| h.alias == st.host) {
                if let Some(tc) = host.tunnels.iter().find(|t| t.name == st.tunnel) {
                    let _ = start_tunnel(&st.host, tc, &mut app.registry);
                }
            }
        }
    }
    app.save_registry();
}

fn save_scenario(app: &mut App) {
    use ssm_core::config::{Scenario, ScenarioTunnel};

    let name = app.scenario_name_buf.trim().to_string();
    if name.is_empty() {
        app.status_message = Some("Scenario name is required".to_string());
        return;
    }

    let all_tunnels = scenario::collect_all_tunnels(&app.config);
    let selected_tunnels: Vec<ScenarioTunnel> = all_tunnels
        .iter()
        .enumerate()
        .filter(|(i, _)| app.scenario_toggle.get(*i).copied().unwrap_or(false))
        .map(|(_, (host, tunnel, _))| ScenarioTunnel {
            host: host.clone(),
            tunnel: tunnel.clone(),
        })
        .collect();

    if selected_tunnels.is_empty() {
        app.status_message = Some("Select at least one tunnel".to_string());
        return;
    }

    app.config.scenarios.push(Scenario {
        name,
        tunnels: selected_tunnels,
    });
    app.save_config();
    app.status_message = Some("Scenario created".to_string());
}
