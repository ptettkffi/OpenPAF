use std::fs;
use std::error::Error;
use std::marker::Sized;
use serde_json::{Value, Map};
use super::super::error::PafError;

pub trait Configuration {
    fn read_from_file(path: &str) -> Result<Self, Box<Error>> where Self: Sized;
    fn read_config(config: &str) -> Result<Self, Box<Error>> where Self: Sized;

    fn as_map(&self) -> Map<String, Value>;
    fn as_json(&self) -> String;
    fn as_text(&self) -> String;
}

/// A general configuration parser. Parses a single JSON object with KVP pairs.
/// Can parse in any depth.
pub struct GeneralConfig {
    config: Map<String, Value>
}

impl Configuration for GeneralConfig {
    /// Reads a JSON configuration file, and create a `GeneralConfig` on
    /// success. If fails, raises an error.
    /// 
    /// ## Arguments
    /// * `path` - Path to the configuration file
    /// 
    /// ## Examples
    /// ```
    /// let res = GeneralConfig::read_from_file("config.json").unwrap();
    /// ```
    fn read_from_file(path: &str) -> Result<GeneralConfig, Box<Error>> {
        let config = fs::read_to_string(path)?;
        GeneralConfig::read_config(&config)
    }

    /// Reads a JSON configuration string, and create a `GeneralConfig` on
    /// success. If fails, raises an error.
    /// 
    /// ## Arguments
    /// * `config` - A valid JSON object string
    /// 
    /// ## Examples
    /// ```
    /// let json = r#"{
    ///         "a": "b",
    ///         "b": 5,
    ///         "c": [1, 2, 3]
    ///     }"#;
    /// let result = GeneralConfig::read_config(json).unwrap();
    /// ```
    fn read_config(config: &str) -> Result<GeneralConfig, Box<Error>> {
        let parsed: Value = serde_json::from_str(config)?;

        let obj = parsed.as_object();
        if let Some(p) = obj {
            Ok(GeneralConfig{ config: p.clone() })
        } else {
            Err(PafError::create_error(&format!("Could not parse configuration as a valid JSON object.")))
        }
    }

    /// Returns the underlying configuration as a `serde_json::Map` object.
    /// 
    /// ## Examples
    /// ```
    /// let map = config.as_map();
    /// println!("There are {} items in the configuration.", map.len());
    /// ```
    fn as_map(&self) -> Map<String, Value> {
        self.config.clone()
    }

    /// Serializes the unerlying configuration to a pretty printed JSON.
    fn as_json(&self) -> String {
        serde_json::to_string_pretty(&self.config).unwrap()
    }

    /// Serializes the unerlying configuration to whitespace delimited key-value pairs.
    /// If a value has depth > 1, serializes the value as a single line JSON string.
    /// 
    /// Using this method the following JSON configuration
    /// ```
    /// {
    ///     "id": 15,
    ///     "name": "John Doe",
    ///     "contacts": ["Susan", "Greg"]
    /// }
    /// ```
    /// becomes
    /// ```
    /// id 15
    /// name John Doe
    /// contacts ["Susan","Greg"]
    /// ```
    fn as_text(&self) -> String {
        // Functional optimization of a simple for loop iterating over KVPs (k, v) in a HashMap and serializing them
        self.as_map().into_iter().fold(
            "".to_string(), |text, (k, v)|
                text + k.as_str() + " " + v.as_str().unwrap_or(&serde_json::to_string(&v).unwrap_or("".to_string())) + "\n"
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

    mod read_config {
        use super::super::*;

        #[test]
        fn reads_json_object() {
            let json = r#"{
                "a": "b",
                "b": 5,
                "c": [1, 2, 3]
            }"#;
            let result = GeneralConfig::read_config(json);
            assert!(result.is_ok());
            let config = result.unwrap();
            assert_eq!(config.config["a"], "b");
            assert_eq!(config.config["b"], 5);
            assert_eq!(config.config["c"].as_array().unwrap().to_vec(), vec!(1, 2, 3));
        }

        #[test]
        fn errs_on_invalid_json() {
            let json = r#"{
                a: "b",
                "b": 5,
                "c": [1, 2, 3]
            }"#;
            let result = GeneralConfig::read_config(json);
            assert!(result.is_err());
        }

        #[test]
        fn errs_on_not_object() {
            let json = "[1, 2, 3]";
            let result = GeneralConfig::read_config(json);
            assert!(result.is_err());
        }
    }

    mod as_map {
        use super::super::*;

        #[test]
        fn returns_a_map() {
            let json = r#"{
                "a": "b",
                "b": 5,
                "c": [1, 2, 3]
            }"#;
            let config = GeneralConfig::read_config(json).unwrap();
            let map = config.as_map();
            assert_eq!(map.len(), 3);
        }
    }

    mod as_json {
        use super::super::*;

        #[test]
        fn returns_pretty_json() {
            let json = "{\n  \"a\": \"b\",\n  \"b\": 5,\n  \"c\": [\n    1,\n    2,\n    3\n  ]\n}";
            let config = GeneralConfig::read_config(json).unwrap();
            let res_json = config.as_json();
            assert_eq!(res_json, json);
        }
    }

    mod as_text {
        use super::super::*;

        #[test]
        fn returns_valid_text_config() {
            let json = r#"{
                "a": "b",
                "b": 5,
                "c": [1, 2, 3]
            }"#;
            let expected = "a b\nb 5\nc [1,2,3]";
            let config = GeneralConfig::read_config(json).unwrap();
            let text = config.as_text();
            assert_eq!(text, expected);
        }
    }
}