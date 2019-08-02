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
            log: Some("openpaf.log".to_string()),
            error_log: None,
            archive_dir: Some("~/archive".to_string()),
            main_server: None,
            servers: None,
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
    ///         "modules": [{
    ///             "name": "dummy",
    ///             "path": "dummy",
    ///             "config": "dummy",
    ///             "mod_type": "Analysis"
    ///         }]
    ///     }"#;
    /// let result = SystemConfig::read_config(json).unwrap();
    /// ```
    fn read_config(config: &str) -> Result<SystemConfig, Box<Error>> {
        let mut parsed: SystemConfig = serde_json::from_str(config)?;
        parsed._fill_defaults();
        Ok(parsed)
    }

    fn as_map(&self) -> Map<String, Value> {
        let json = self.as_json();
        let genconf = GeneralConfig::read_config(&json).unwrap();
        genconf.as_map()
    }

    fn as_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }

    fn as_text(&self) -> String {
        let json = self.as_json();
        let genconf = GeneralConfig::read_config(&json).unwrap();
        genconf.as_text()
    }
}

impl SystemConfig {
    fn _fill_defaults(&mut self) {
        let defaults: SystemConfig = Default::default();

        if self.archive_dir.is_none() {
            self.archive_dir = defaults.archive_dir;
        }
        if self.io_timeout.is_none() {
            self.io_timeout = defaults.io_timeout;
        }
        if self.analysis_timeout.is_none() {
            self.analysis_timeout = defaults.analysis_timeout;
        }
    }
}