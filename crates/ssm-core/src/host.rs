use crate::config::{Config, Host};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostError {
    #[error("host alias '{0}' already exists")]
    DuplicateAlias(String),
    #[error("host alias '{0}' not found")]
    NotFound(String),
}

impl Config {
    pub fn add_host(&mut self, host: Host) -> Result<(), HostError> {
        if self.hosts.iter().any(|h| h.alias == host.alias) {
            return Err(HostError::DuplicateAlias(host.alias));
        }
        self.hosts.push(host);
        Ok(())
    }

    pub fn remove_host(&mut self, alias: &str) -> Result<Host, HostError> {
        let idx = self
            .hosts
            .iter()
            .position(|h| h.alias == alias)
            .ok_or_else(|| HostError::NotFound(alias.into()))?;
        Ok(self.hosts.remove(idx))
    }

    pub fn update_host(&mut self, alias: &str, updated: Host) -> Result<(), HostError> {
        let idx = self
            .hosts
            .iter()
            .position(|h| h.alias == alias)
            .ok_or_else(|| HostError::NotFound(alias.into()))?;
        if alias != updated.alias && self.hosts.iter().any(|h| h.alias == updated.alias) {
            return Err(HostError::DuplicateAlias(updated.alias));
        }
        self.hosts[idx] = updated;
        Ok(())
    }

    pub fn find_host(&self, alias: &str) -> Option<&Host> {
        self.hosts.iter().find(|h| h.alias == alias)
    }

    pub fn find_host_mut(&mut self, alias: &str) -> Option<&mut Host> {
        self.hosts.iter_mut().find(|h| h.alias == alias)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn make_host(alias: &str) -> Host {
        Host {
            alias: alias.into(),
            hostname: "1.2.3.4".into(),
            user: None,
            port: 22,
            identity_file: None,
            tags: vec![],
            notes: None,
            tunnels: vec![],
            commands: vec![],
        }
    }

    #[test]
    fn test_add_host() {
        let mut config = Config::default();
        config.add_host(make_host("server1")).unwrap();
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].alias, "server1");
    }

    #[test]
    fn test_add_duplicate_alias_fails() {
        let mut config = Config::default();
        config.add_host(make_host("server1")).unwrap();
        let err = config.add_host(make_host("server1")).unwrap_err();
        assert!(matches!(err, HostError::DuplicateAlias(_)));
    }

    #[test]
    fn test_remove_host() {
        let mut config = Config::default();
        config.add_host(make_host("server1")).unwrap();
        config.add_host(make_host("server2")).unwrap();
        let removed = config.remove_host("server1").unwrap();
        assert_eq!(removed.alias, "server1");
        assert_eq!(config.hosts.len(), 1);
    }

    #[test]
    fn test_remove_nonexistent_fails() {
        let mut config = Config::default();
        let err = config.remove_host("nope").unwrap_err();
        assert!(matches!(err, HostError::NotFound(_)));
    }

    #[test]
    fn test_update_host() {
        let mut config = Config::default();
        config.add_host(make_host("server1")).unwrap();
        let mut updated = make_host("server1");
        updated.hostname = "5.6.7.8".into();
        config.update_host("server1", updated).unwrap();
        assert_eq!(config.hosts[0].hostname, "5.6.7.8");
    }

    #[test]
    fn test_update_host_rename() {
        let mut config = Config::default();
        config.add_host(make_host("old-name")).unwrap();
        let renamed = make_host("new-name");
        config.update_host("old-name", renamed).unwrap();
        assert_eq!(config.hosts[0].alias, "new-name");
    }

    #[test]
    fn test_update_host_rename_to_existing_fails() {
        let mut config = Config::default();
        config.add_host(make_host("server1")).unwrap();
        config.add_host(make_host("server2")).unwrap();
        let renamed = make_host("server2");
        let err = config.update_host("server1", renamed).unwrap_err();
        assert!(matches!(err, HostError::DuplicateAlias(_)));
    }

    #[test]
    fn test_find_host() {
        let mut config = Config::default();
        config.add_host(make_host("server1")).unwrap();
        assert!(config.find_host("server1").is_some());
        assert!(config.find_host("nope").is_none());
    }
}
