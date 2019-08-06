use std::fs;
use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value, Map};
use super::super::server::Server;
use super::super::module::Module;
use super::config::{GeneralConfig, Configuration};

/// A strongly typed system configuration required for the OpenPAF binary.
#[derive(Deserialize, Serialize)]
pub struct SystemConfig {
    pub modules: Vec<Module>,
    pub log: Option<String>,
    pub error_log: Option<String>,
    pub archive_dir: Option<String>,
    pub main_server: Option<Server>,
    pub servers: Option<Vec<Server>>,
    pub io_timeout: Option<u64>,
    pub analysis_timeout: Option<u64>,
}

/// Default system configuration. Only used for filling in some optional parameters.
/// Required parameters are filled with dummy values. DO NOT USE THEM!
impl Default for SystemConfig {
    fn default() -> SystemConfig {
        SystemConfig {
            log: Some("/var/log/openpaf/openpaf.log".to_string()),
            error_log: None,
            archive_dir: Some("~/.openpaf/archive".to_string()),
            main_server: None,
            servers: Some(vec![]),
            modules: vec![Default::default()],
            io_timeout: Some(300),
            analysis_timeout: Some(600)
        }
    }
}

impl Configuration for SystemConfig {
    /// Reads a JSON configuration file, and create a `SystemConfig` on
    /// success. If fails, raises an error.
    /// 
    /// ## Arguments
    /// * `path` - Path to the configuration file
    /// 
    /// ## Examples
    /// ```
    /// let res = SystemConfig::read_from_file("config.json").unwrap();
    /// ```
    fn read_from_file(path: &str) -> Result<SystemConfig, Box<Error>> {
        let config = fs::read_to_string(path)?;
        SystemConfig::read_config(&config)
    }

    /// Reads a JSON configuration string, and create a `SystemConfig` on
    /// success. If fails, raises an error.
    /// 
    /// ## Arguments
    /// * `config` - A valid JSON object string
    /// 
    /// ## Examples
    /// ```
    /// let json = r#"{
    ///     "modules": [{
    ///         "name": "dummy",
    ///         "path": "dummy",
    ///         "config": "dummy",
    ///         "mod_type": "Analysis"
    ///     }]
    /// }"#;
    /// let result = SystemConfig::read_config(json).unwrap();
    /// ```
    fn read_config(config: &str) -> Result<SystemConfig, Box<Error>> {
        let mut parsed: SystemConfig = serde_json::from_str(config)?;
        parsed._fill_defaults();
        parsed._sanitize_servers();
        Ok(parsed)
    }

    /// Returns the underlying configuration as a `serde_json::Map` object.
    /// 
    /// ## Examples
    /// ```
    /// let map = config.as_map();
    /// println!("There are {} items in the configuration.", map.len());
    /// ```
    fn as_map(&self) -> Map<String, Value> {
        let json = self.as_json();
        let genconf = GeneralConfig::read_config(&json).unwrap();
        genconf.as_map()
    }

    /// Serializes the unerlying configuration to a pretty printed JSON.
    fn as_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }

    /// Serializes the unerlying configuration to whitespace delimited key-value pairs.
    /// If a value has depth > 1, serializes the value as a single line JSON string.
    /// 
    /// Using this method will result in parsing overhead. Use `SystemConfig::as_json`
    /// instead.
    fn as_text(&self) -> String {
        let json = self.as_json();
        let genconf = GeneralConfig::read_config(&json).unwrap();
        genconf.as_text()
    }
}

impl SystemConfig {
    /// Fills optional system configurations with default values, if absent.
    fn _fill_defaults(&mut self) {
        let defaults: SystemConfig = Default::default();

        if self.log.is_none() {
            self.log = defaults.log;
        }
        if self.archive_dir.is_none() {
            self.archive_dir = defaults.archive_dir;
        }
        if self.servers.is_none() {
            self.servers = defaults.servers;
        }
        if self.io_timeout.is_none() {
            self.io_timeout = defaults.io_timeout;
        }
        if self.analysis_timeout.is_none() {
            self.analysis_timeout = defaults.analysis_timeout;
        }
    }

    /// Adds the main server to the server list, and removes duplicates.
    fn _sanitize_servers(&mut self) {
        if let Some(server) = &self.main_server {
            if let Some(serverlist) = &mut self.servers {
                serverlist.push(server.clone());
                Server::remove_duplicates(serverlist);
            }
        }
    }
}

#[cfg(test)]
mod test {
    mod read_from_file {
        use super::super::*;

        #[test]
        fn reads_from_file() {
            let res = SystemConfig::read_from_file("test/sysconfig_full.json");
            assert!(res.is_ok());
        }
    }

    mod read_config {
        use super::super::*;

        #[test]
        fn does_not_need_optional_params() {
            let conf = r#"{
                "modules": [{
                    "name": "",
                    "path": "",
                    "config": "",
                    "mod_type": "Analysis"
                }]
            }"#;

            let sysconf = SystemConfig::read_config(conf);
            assert!(sysconf.is_ok());
        }

        #[test]
        fn enforces_required_params() {
            let conf = r#"{
                "log": "test.log",
                "error_log": "error.log",
                "archive_dir": "archive"
            }"#;

            let sysconf = SystemConfig::read_config(conf);
            assert!(sysconf.is_err());
        }

        #[test]
        fn dumps_extra_params() {
            let conf = r#"{
                "modules": [{
                    "name": "",
                    "path": "",
                    "config": "",
                    "mod_type": "Analysis"
                }],
                "extra": 11
            }"#;

            let sysconf = SystemConfig::read_config(conf);
            assert!(sysconf.is_ok());
        }

        #[test]
        fn fills_optional_params() {
            let conf = r#"{
                "modules": [{
                    "name": "",
                    "path": "",
                    "config": "",
                    "mod_type": "Analysis"
                }],
                "extra": 11
            }"#;

            let sysconf = SystemConfig::read_config(conf).unwrap();
            assert!(sysconf.log.is_some());
            assert!(sysconf.archive_dir.is_some());
            assert!(sysconf.servers.is_some());
            assert!(sysconf.io_timeout.is_some());
            assert!(sysconf.analysis_timeout.is_some());
        }

        #[test]
        fn reads_servers() {
            let conf = r#"{
                "modules": [{
                    "name": "",
                    "path": "",
                    "config": "",
                    "mod_type": "Analysis"
                }],
                "main_server": {
                    "name": "me",
                    "ip": "127.0.0.1",
                    "ssh_port": 22
                },
                "servers": [{
                        "name": "nextone",
                        "ip": "169.0.0.1",
                        "ssh_port": 22
                }]
            }"#;

            let sysconf = SystemConfig::read_config(conf).unwrap();
            let main = sysconf.main_server.unwrap();
            let servers = sysconf.servers.unwrap();

            assert_eq!(main.name.unwrap(), "me".to_string());
            assert_eq!(main.ip, "127.0.0.1".to_string());
            assert_eq!(main.ssh_port, 22);

            assert_eq!(servers.len(), 2);
            assert_eq!(servers[1].name.as_ref().unwrap(), "nextone");
            assert_eq!(servers[1].ip, "169.0.0.1".to_string());
            assert_eq!(servers[1].ssh_port, 22);
            assert_eq!(servers[0].ip, "127.0.0.1".to_string());
        }
    }

    mod as_map {
        use super::super::*;

        #[test]
        fn it_works() {
            let conf = r#"{
                "modules": [{
                    "name": "",
                    "path": "",
                    "config": "",
                    "mod_type": "Analysis"
                }]
            }"#;

            let sysconf = SystemConfig::read_config(conf).unwrap();
            sysconf.as_map();
        }
    }

    mod as_json {
        use super::super::*;

        #[test]
        fn it_works() {
            let conf = r#"{
                "modules": [{
                    "name": "",
                    "path": "",
                    "config": "",
                    "mod_type": "Analysis"
                }]
            }"#;

            let sysconf = SystemConfig::read_config(conf).unwrap();
            sysconf.as_json();
        }
    }

    mod as_text {
        use super::super::*;

        #[test]
        fn it_works() {
            let conf = r#"{
                "modules": [{
                    "name": "",
                    "path": "",
                    "config": "",
                    "mod_type": "Analysis"
                }]
            }"#;

            let sysconf = SystemConfig::read_config(conf).unwrap();
            sysconf.as_text();
        }
    }

    mod _fill_defaults {
        use super::super::*;

        #[test]
        fn fills_correct_values() {
            let mut sysconf = SystemConfig{
                modules: vec![Default::default()],
                log: None,
                error_log: None,
                archive_dir: None,
                main_server: None,
                servers: None,
                io_timeout: None,
                analysis_timeout: None
             };
             let default = SystemConfig{..Default::default()};

             sysconf._fill_defaults();

             assert_eq!(sysconf.log.unwrap(), default.log.unwrap());
             assert!(sysconf.error_log.is_none());
             assert_eq!(sysconf.archive_dir.unwrap(), default.archive_dir.unwrap());
             assert!(sysconf.main_server.is_none());
             assert_eq!(sysconf.servers.unwrap().len(), default.servers.unwrap().len());
             assert_eq!(sysconf.io_timeout.unwrap(), default.io_timeout.unwrap());
             assert_eq!(sysconf.analysis_timeout.unwrap(), default.analysis_timeout.unwrap());
        }
    }

    mod _sanitize_servers {
        use super::super::*;

        #[test]
        fn adds_main_to_server_list() {
            let mut sysconf = SystemConfig{
                main_server: Some(Server {
                    name: Some("me".to_string()),
                    ip: "127.0.0.1".to_string(),
                    ssh_port: 22
                }),
                servers: Some(vec![
                    Server {
                        name: Some("nextone".to_string()),
                        ip: "192.16.1.1".to_string(),
                        ssh_port: 22
                    }
                ]),
                ..Default::default()
             };
             sysconf._sanitize_servers();

             assert_eq!(sysconf.servers.unwrap().len(), 2);
        }

        #[test]
        fn removes_duplicates() {
            let mut sysconf = SystemConfig{
                main_server: Some(Server {
                    name: Some("me".to_string()),
                    ip: "127.0.0.1".to_string(),
                    ssh_port: 22
                }),
                servers: Some(vec![
                    Server {
                        name: Some("nextone".to_string()),
                        ip: "192.16.1.1".to_string(),
                        ssh_port: 22
                    },
                    Server {
                        name: Some("me".to_string()),
                        ip: "127.0.0.1".to_string(),
                        ssh_port: 22
                    },
                    Server {
                        name: Some("nextone".to_string()),
                        ip: "192.16.1.1".to_string(),
                        ssh_port: 22
                    }
                ]),
                ..Default::default()
             };
             sysconf._sanitize_servers();

             assert_eq!(sysconf.servers.unwrap().len(), 2);
        }
    }
}