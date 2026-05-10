use ssm_core::config::{CommandConfig, Config, Host, TunnelConfig};
use ssm_core::tunnel::TunnelRegistry;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Filter,
    Wizard(WizardState),
    EditHost(WizardState),
    Confirm(ConfirmAction),
    CommandPicker,
    OutputViewer,
    TunnelMenu,
    TunnelWizard(TunnelWizardState),
    CommandWizard(CommandWizardState),
    ImportPaste,
    ImportPreview,
    ScenarioMenu,
    ScenarioCreate,
    Help,
}

// ---------------------------------------------------------------------------
// WizardStep
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum WizardStep {
    Alias,
    Hostname,
    User,
    Port,
    IdentityFile,
    Tags,
}

// ---------------------------------------------------------------------------
// WizardState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct WizardState {
    pub step: WizardStep,
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub tags: String,
    /// Preserved from the original host when editing — not shown in the wizard UI.
    pub original_tunnels: Vec<TunnelConfig>,
    pub original_commands: Vec<CommandConfig>,
}

impl WizardState {
    pub fn new() -> Self {
        Self {
            step: WizardStep::Alias,
            alias: String::new(),
            hostname: String::new(),
            user: whoami::username(),
            port: "22".to_string(),
            identity_file: String::new(),
            tags: String::new(),
            original_tunnels: vec![],
            original_commands: vec![],
        }
    }

    pub fn from_host(host: &Host) -> Self {
        Self {
            step: WizardStep::Alias,
            alias: host.alias.clone(),
            hostname: host.hostname.clone(),
            user: host.user.clone().unwrap_or_default(),
            port: host.port.to_string(),
            identity_file: host
                .identity_file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            tags: host.tags.join(", "),
            original_tunnels: host.tunnels.clone(),
            original_commands: host.commands.clone(),
        }
    }

    pub fn current_value(&self) -> &str {
        match self.step {
            WizardStep::Alias => &self.alias,
            WizardStep::Hostname => &self.hostname,
            WizardStep::User => &self.user,
            WizardStep::Port => &self.port,
            WizardStep::IdentityFile => &self.identity_file,
            WizardStep::Tags => &self.tags,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.step {
            WizardStep::Alias => &mut self.alias,
            WizardStep::Hostname => &mut self.hostname,
            WizardStep::User => &mut self.user,
            WizardStep::Port => &mut self.port,
            WizardStep::IdentityFile => &mut self.identity_file,
            WizardStep::Tags => &mut self.tags,
        }
    }

    pub fn step_label(&self) -> &'static str {
        match self.step {
            WizardStep::Alias => "Alias",
            WizardStep::Hostname => "Hostname",
            WizardStep::User => "User",
            WizardStep::Port => "Port",
            WizardStep::IdentityFile => "Identity File",
            WizardStep::Tags => "Tags",
        }
    }

    pub fn next_step(&mut self) {
        self.step = match self.step {
            WizardStep::Alias => WizardStep::Hostname,
            WizardStep::Hostname => WizardStep::User,
            WizardStep::User => WizardStep::Port,
            WizardStep::Port => WizardStep::IdentityFile,
            WizardStep::IdentityFile => WizardStep::Tags,
            WizardStep::Tags => WizardStep::Tags, // last step
        };
    }

    pub fn prev_step(&mut self) {
        self.step = match self.step {
            WizardStep::Alias => WizardStep::Alias, // first step
            WizardStep::Hostname => WizardStep::Alias,
            WizardStep::User => WizardStep::Hostname,
            WizardStep::Port => WizardStep::User,
            WizardStep::IdentityFile => WizardStep::Port,
            WizardStep::Tags => WizardStep::IdentityFile,
        };
    }
}

// ---------------------------------------------------------------------------
// TunnelWizardState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelWizardStep {
    Name,
    LocalPort,
    RemoteHost,
    RemotePort,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TunnelWizardState {
    pub step: TunnelWizardStep,
    pub name: String,
    pub local_port: String,
    pub remote_host: String,
    pub remote_port: String,
}

impl TunnelWizardState {
    pub fn new() -> Self {
        Self {
            step: TunnelWizardStep::Name,
            name: String::new(),
            local_port: String::new(),
            remote_host: "localhost".to_string(),
            remote_port: String::new(),
        }
    }

    pub fn current_value(&self) -> &str {
        match self.step {
            TunnelWizardStep::Name => &self.name,
            TunnelWizardStep::LocalPort => &self.local_port,
            TunnelWizardStep::RemoteHost => &self.remote_host,
            TunnelWizardStep::RemotePort => &self.remote_port,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.step {
            TunnelWizardStep::Name => &mut self.name,
            TunnelWizardStep::LocalPort => &mut self.local_port,
            TunnelWizardStep::RemoteHost => &mut self.remote_host,
            TunnelWizardStep::RemotePort => &mut self.remote_port,
        }
    }

    pub fn step_label(&self) -> &'static str {
        match self.step {
            TunnelWizardStep::Name => "Name",
            TunnelWizardStep::LocalPort => "Local Port",
            TunnelWizardStep::RemoteHost => "Remote Host",
            TunnelWizardStep::RemotePort => "Remote Port",
        }
    }

    pub fn next_step(&mut self) {
        self.step = match self.step {
            TunnelWizardStep::Name => TunnelWizardStep::LocalPort,
            TunnelWizardStep::LocalPort => TunnelWizardStep::RemoteHost,
            TunnelWizardStep::RemoteHost => TunnelWizardStep::RemotePort,
            TunnelWizardStep::RemotePort => TunnelWizardStep::RemotePort,
        };
    }

    pub fn prev_step(&mut self) {
        self.step = match self.step {
            TunnelWizardStep::Name => TunnelWizardStep::Name,
            TunnelWizardStep::LocalPort => TunnelWizardStep::Name,
            TunnelWizardStep::RemoteHost => TunnelWizardStep::LocalPort,
            TunnelWizardStep::RemotePort => TunnelWizardStep::RemoteHost,
        };
    }

    pub fn step_index(&self) -> usize {
        match self.step {
            TunnelWizardStep::Name => 0,
            TunnelWizardStep::LocalPort => 1,
            TunnelWizardStep::RemoteHost => 2,
            TunnelWizardStep::RemotePort => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// CommandWizardState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CommandWizardStep {
    Name,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandWizardState {
    pub step: CommandWizardStep,
    pub name: String,
    pub command: String,
}

impl CommandWizardState {
    pub fn new() -> Self {
        Self {
            step: CommandWizardStep::Name,
            name: String::new(),
            command: String::new(),
        }
    }

    pub fn current_value(&self) -> &str {
        match self.step {
            CommandWizardStep::Name => &self.name,
            CommandWizardStep::Command => &self.command,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.step {
            CommandWizardStep::Name => &mut self.name,
            CommandWizardStep::Command => &mut self.command,
        }
    }

    pub fn step_label(&self) -> &'static str {
        match self.step {
            CommandWizardStep::Name => "Name",
            CommandWizardStep::Command => "Command",
        }
    }

    pub fn step_index(&self) -> usize {
        match self.step {
            CommandWizardStep::Name => 0,
            CommandWizardStep::Command => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// ConfirmAction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteHost(String),
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    pub registry: TunnelRegistry,
    pub registry_path: PathBuf,
    pub mode: Mode,
    pub selected_index: usize,
    pub filter_text: String,
    pub filtered_indices: Vec<usize>,
    pub command_selected: usize,
    pub tunnel_selected: usize,
    pub output_text: String,
    pub output_scroll: u16,
    pub should_quit: bool,
    pub pending_ssh: Option<Vec<String>>,
    pub status_message: Option<String>,
    pub import_buffer: String,
    pub import_parsed: Vec<ssm_core::import::ParsedHost>,
    pub import_scroll: u16,
    pub scenario_selected: usize,
    pub scenario_name_buf: String,
    pub scenario_toggle: Vec<bool>,
}

impl App {
    pub fn new(
        config: Config,
        config_path: PathBuf,
        registry: TunnelRegistry,
        registry_path: PathBuf,
    ) -> Self {
        let filtered_indices = (0..config.hosts.len()).collect();
        Self {
            config,
            config_path,
            registry,
            registry_path,
            mode: Mode::Normal,
            selected_index: 0,
            filter_text: String::new(),
            filtered_indices,
            command_selected: 0,
            tunnel_selected: 0,
            output_text: String::new(),
            output_scroll: 0,
            should_quit: false,
            pending_ssh: None,
            status_message: None,
            import_buffer: String::new(),
            import_parsed: vec![],
            import_scroll: 0,
            scenario_selected: 0,
            scenario_name_buf: String::new(),
            scenario_toggle: vec![],
        }
    }

    pub fn selected_host(&self) -> Option<&Host> {
        let idx = self.filtered_indices.get(self.selected_index)?;
        self.config.hosts.get(*idx)
    }

    pub fn selected_host_alias(&self) -> Option<&str> {
        self.selected_host().map(|h| h.alias.as_str())
    }

    pub fn save_config(&mut self) {
        if let Err(e) = self.config.save(&self.config_path) {
            eprintln!("failed to save config: {}", e);
            self.status_message = Some(format!("Error saving config: {}", e));
        }
    }

    pub fn save_registry(&mut self) {
        if let Err(e) = self.registry.save(&self.registry_path) {
            eprintln!("failed to save registry: {}", e);
            self.status_message = Some(format!("Error saving registry: {}", e));
        }
    }

    /// Rebuild filtered_indices from the current filter_text.
    /// Supports "tag:xxx" prefix and substring matching on alias/hostname/tags.
    pub fn update_filter(&mut self) {
        let query = self.filter_text.trim().to_lowercase();

        self.filtered_indices = if query.is_empty() {
            (0..self.config.hosts.len()).collect()
        } else if let Some(tag_query) = query.strip_prefix("tag:") {
            let tag_query = tag_query.trim();
            self.config
                .hosts
                .iter()
                .enumerate()
                .filter(|(_, h)| {
                    h.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(tag_query))
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            self.config
                .hosts
                .iter()
                .enumerate()
                .filter(|(_, h)| {
                    h.alias.to_lowercase().contains(&query)
                        || h.hostname.to_lowercase().contains(&query)
                        || h.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .map(|(i, _)| i)
                .collect()
        };

        // Clamp selection
        if self.filtered_indices.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = self.filtered_indices.len() - 1;
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        let max = self.filtered_indices.len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }
}
