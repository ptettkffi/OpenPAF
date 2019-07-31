use std::fs;
use std::error::Error;
use std::marker::Sized;
use serde_json::{Value, Map};
use super::super::error::PafError;

pub trait Configuration {
    fn read_from_file(path: &str) -> Result<Self, Box<Error>> where Self: Sized;
    fn read_config(config: &str) -> Result<Self, Box<Error>> where Self: Sized;

    fn as_map(&self) -> &Map<String, Value>;
    fn as_json(&self) -> String;
    fn as_text(&self) -> String;
}

pub struct GeneralConfig {
    config: Map<String, Value>
}

impl Configuration for GeneralConfig {
    fn read_from_file(path: &str) -> Result<GeneralConfig, Box<Error>> {
        let config = fs::read_to_string(path)?;
        GeneralConfig::read_config(&config)
    }

    fn read_config(config: &str) -> Result<GeneralConfig, Box<Error>> {
        let parsed: Value = serde_json::from_str(config)?;

        let obj = parsed.as_object();
        if let Some(p) = obj {
            Ok(GeneralConfig{ config: p.clone() })
        } else {
            Err(PafError::create_error(&format!("Could not parse configuration as a valid JSON object.")))
        }
    }

    fn as_map(&self) -> &Map<String, Value> {
        &self.config
    }

    fn as_json(&self) -> String {
        serde_json::to_string_pretty(&self.config).unwrap()
    }

    fn as_text(&self) -> String {
        // Functional optimization of a simple for loop iterating over KVPs (k, v) in a HashMap and serializing them
        self.as_map().into_iter().fold(
            "".to_string(), |text, (k, v)|
                text + k.as_str() + " " + v.as_str().unwrap_or(&serde_json::to_string(v).unwrap_or("".to_string())) + "\n"
        ).trim().to_string()
    }
}

#[cfg(test)]
mod test {
    mod read_from_file {
        use super::super::*;

        #[test]
        fn can_read_from_file() {
            let res = GeneralConfig::read_from_file("test/config.json");
            assert!(res.is_ok())
        }
    }
}