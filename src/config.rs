use std::path::Path;

use serde::Deserialize;

use crate::magic_packet::MagicPacketError;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_broadcast")]
    pub broadcast: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default)]
    pub machines: Vec<Machine>,
    #[serde(default)]
    pub schedules: Vec<Schedule>,
    /// Base URL of the runetale dashboard. Set it to offer a remote desktop
    /// link for machines that carry a `runetale_ip`.
    #[serde(default)]
    pub runetale_dashboard: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Machine {
    pub name: String,
    pub mac: String,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub broadcast: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    /// Overlay address this machine answers on inside the mesh. Distinct from
    /// `ip`, which is the LAN address used to wake and ping it.
    #[serde(default)]
    pub runetale_ip: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Schedule {
    pub machine: String,
    pub cron: String,
}

fn default_broadcast() -> String {
    "255.255.255.255".to_string()
}

fn default_port() -> u16 {
    9
}

fn default_listen() -> String {
    "0.0.0.0:7777".to_string()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, MagicPacketError> {
        let content = std::fs::read_to_string(path).map_err(MagicPacketError::Io)?;
        let config: Config = serde_yaml::from_str(&content)
            .map_err(|e| MagicPacketError::InvalidConfig(e.to_string()))?;
        Ok(config)
    }

    pub fn find_machine(&self, name: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.name == name)
    }

    pub fn resolve_broadcast(&self, machine: &Machine) -> String {
        let addr = machine.broadcast.as_deref().unwrap_or(&self.broadcast);
        let port = machine.port.unwrap_or(self.port);
        format!("{}:{}", addr, port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            broadcast: default_broadcast(),
            port: default_port(),
            listen: default_listen(),
            machines: Vec::new(),
            schedules: Vec::new(),
            runetale_dashboard: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_config() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
broadcast: "192.168.1.255"
port: 7
machines:
  - name: desktop
    mac: "aa:bb:cc:dd:ee:ff"
    ip: "192.168.1.100"
  - name: server
    mac: "11:22:33:44:55:66"
    broadcast: "10.0.0.255"
    port: 9
"#
        )
        .unwrap();

        let config = Config::load(f.path()).unwrap();
        assert_eq!(config.broadcast, "192.168.1.255");
        assert_eq!(config.port, 7);
        assert_eq!(config.machines.len(), 2);
        assert_eq!(config.machines[0].name, "desktop");
        assert_eq!(config.machines[1].broadcast.as_deref(), Some("10.0.0.255"));
    }

    #[test]
    fn test_defaults() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "machines: []").unwrap();

        let config = Config::load(f.path()).unwrap();
        assert_eq!(config.broadcast, "255.255.255.255");
        assert_eq!(config.port, 9);
        assert_eq!(config.listen, "0.0.0.0:7777");
    }

    #[test]
    fn test_find_machine() {
        let config = Config {
            machines: vec![Machine {
                name: "test".to_string(),
                mac: "aa:bb:cc:dd:ee:ff".to_string(),
                ip: None,
                broadcast: None,
                port: None,
                runetale_ip: None,
            }],
            ..Default::default()
        };
        assert!(config.find_machine("test").is_some());
        assert!(config.find_machine("nope").is_none());
    }

    #[test]
    fn test_resolve_broadcast_default() {
        let config = Config::default();
        let machine = Machine {
            name: "x".to_string(),
            mac: "aa:bb:cc:dd:ee:ff".to_string(),
            ip: None,
            broadcast: None,
            port: None,
            runetale_ip: None,
        };
        assert_eq!(config.resolve_broadcast(&machine), "255.255.255.255:9");
    }

    #[test]
    fn test_resolve_broadcast_override() {
        let config = Config::default();
        let machine = Machine {
            name: "x".to_string(),
            mac: "aa:bb:cc:dd:ee:ff".to_string(),
            ip: None,
            broadcast: Some("10.0.0.255".to_string()),
            port: Some(7),
            runetale_ip: None,
        };
        assert_eq!(config.resolve_broadcast(&machine), "10.0.0.255:7");
    }

    #[test]
    fn test_load_with_schedules() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
machines:
  - name: desktop
    mac: "aa:bb:cc:dd:ee:ff"
schedules:
  - machine: desktop
    cron: "0 30 8 * * Mon-Fri"
"#
        )
        .unwrap();

        let config = Config::load(f.path()).unwrap();
        assert_eq!(config.schedules.len(), 1);
        assert_eq!(config.schedules[0].machine, "desktop");
        assert_eq!(config.schedules[0].cron, "0 30 8 * * Mon-Fri");
    }
}
