use std::fs;
use std::panic;
use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value, Map, json};
use postgres::{Connection as PostgresConnection, TlsMode as PostgresTlsMode};
use super::config::{GeneralConfig, Configuration};
use super::super::error::PafError;

/// Enum for the three supported backends by OpenPAF.
#[derive(Deserialize, Serialize)]
enum DatabaseType {
    SQLite,
    MySQL,
    PostgreSQL
}

/// A strongly typed module configuration with space for weakly typed elements.
#[derive(Deserialize, Serialize)]
pub struct ModuleConfig {
    pub timeout: Option<u32>,
    db: Option<DatabaseType>,
    connection_string: Option<String>,
    params: Option<Map<String, Value>>
}

impl Configuration for ModuleConfig {
    fn read_from_file(path: &str) -> Result<ModuleConfig, Box<Error>> {
        let config = fs::read_to_string(path)?;
        ModuleConfig::read_config(&config)
    }

    fn read_config(config: &str) -> Result<ModuleConfig, Box<Error>> {
        let mut parsed: ModuleConfig = serde_json::from_str(config)?;
        parsed._read_db_params()?;
        Ok(parsed)
    }

    fn as_map(&self) -> Map<String, Value> {
        self.params.clone().unwrap_or(Map::new())
    }

    fn as_json(&self) -> String {
        serde_json::to_string_pretty(&self.params).unwrap()
    }

    fn as_text(&self) -> String {
        let json = self.as_json();
        let genconf = GeneralConfig::read_config(&json).unwrap();
        genconf.as_text()
    }
}

impl ModuleConfig {
    fn _read_db_params(&mut self) -> Result<(), Box<Error>> {
        if let Some(db) = &self.db {
            if self.connection_string.is_none() {
                return Err(PafError::create_error("There is no connection string supplied."));
            }

            match db {
                DatabaseType::PostgreSQL => self._fill_with_postgres()?,
                DatabaseType::MySQL => self._fill_with_mysql()?,
                DatabaseType::SQLite => self._fill_with_sqlite()?
            }
        }

        Ok(())
    }

    fn _read_db_string(db_str: &str) -> Option<Vec<&str>> {
        if db_str.starts_with("db:") {
            let db_vec: Vec<&str> = db_str.split(":").collect();
            let db_info: Vec<&str> = db_vec[1].split("/").collect();
            if db_info.len() == 4 {
                return Some(db_info);
            }
        }
        None
    }

    fn _fill_with_postgres(&mut self) -> Result<(), Box<Error>> {
        let cstr = format!("postgresql://{}", self.connection_string.as_ref().unwrap());
        let conn = PostgresConnection::connect(cstr, PostgresTlsMode::None)?;
        let mut filled = self.as_map();

        for (k, v) in self.as_map() {
            if let Some(val) = v.as_str() {
                if let Some(info) = ModuleConfig::_read_db_string(val) {
                    let query = format!("SELECT {} FROM {} WHERE {} = {}", info[0], info[1], info[2], info[3]);
                    let result = &conn.query(&query, &[])?;
                    if result.len() == 1 {
                        let test_type = panic::catch_unwind(|| {
                            let _: String = result.get(0).get(0);
                        });
                        if test_type.is_ok() {
                            let result_val: String = result.get(0).get(0);
                            filled[&k] = json!(result_val);
                        } else {
                            return Err(PafError::create_error(&format!("Invalid type found with query {}", query)));
                        }
                    }
                }
            }
        }
        self.params = Some(filled);
        Ok(())
    }

    fn _fill_with_mysql(&mut self) -> Result<(), Box<Error>> {
        Ok(())
    }

    fn _fill_with_sqlite(&mut self) -> Result<(), Box<Error>> {
        Ok(())
    }

    pub fn merge(&mut self, other: ModuleConfig) {
        let mut merged = self.as_map();
        for (k, v) in other.as_map() {
            merged[&k] = v;
        }

        self.params = Some(merged);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn check_postgres_connection() -> bool {
        // In order to not fail PostgreSQL tests, create a local server structure with the following parameters:
        // Database: openpaf
        // Username: openpaf_user (must be a login user with select privilege to the openpaf table)
        // Password: openpaf123
        // Port: 5433
        // Table: openpaf
        // Columns: id (int), param (varchar), numeric (int), nullable (varchar)
        // Add at least one row with VALUES (0, 'value', 12, NULL)
        let cstr = "postgresql://openpaf_user:openpaf123@localhost:5433/openpaf";
        let conn_res = PostgresConnection::connect(cstr, PostgresTlsMode::None);
        if conn_res.is_err() {
            panic!("Could not connect to PostgreSQL Server.");
        }

        let conn = conn_res.unwrap();
        let query_res = &conn.query("SELECT * FROM openpaf WHERE id = 0", &[]);
        if query_res.is_err() {
            panic!("Could not query table openpaf with column id.");
        }
        true
    }

    mod read_from_file {
        use super::super::*;

        #[test]
        fn reads_from_file() {
            let res = ModuleConfig::read_from_file("test/moduleconfig.json");
            assert!(res.is_ok());
        }
    }

    mod _fill_with_postgres {
        use super::super::*;
        use super::*;

        #[test]
        fn check_connection() {
            assert!(check_postgres_connection())
        }
    }
}