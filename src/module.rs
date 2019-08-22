use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum ModuleType {
    Input,
    Analysis,
    Output
}

#[derive(Deserialize, Serialize)]
pub struct Module {
    pub name: String,
    pub path: Option<String>,
    pub config: Option<String>,
    pub mod_type: ModuleType
}

/// A default dummy module for system config.
impl Default for Module {
    fn default() -> Module {
        Module {
            name: "dummy".to_string(),
            path: None,
            config: None,
            mod_type: ModuleType::Analysis
        }
    }
}