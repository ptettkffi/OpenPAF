use std::fs;
use std::panic;
use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value, Map, json};
use postgres::{Connection as PostgresConnection, TlsMode as PostgresTlsMode};
use postgres::rows::Row;
use postgres::types::FromSql;
use sqlite;
use mysql;
use mysql::consts::ColumnType;
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
                    let query = format!("SELECT {} FROM {} WHERE {} = {}", info[1], info[0], info[2], info[3]);
                    let result = &conn.query(&query, &[])?;
                    if result.len() != 0 {
                        // Try to parse value. Supported types in order: String, i32, f32, f64, i64, bool.
                        let row = result.get(0);
                        if ModuleConfig::_postgres_try_parse::<String>(&row) {
                            let result_val: Option<String> = result.get(0).get(0);
                            filled[&k] = json!(result_val);
                        } else if ModuleConfig::_postgres_try_parse::<i32>(&row) {
                            let result_val: Option<i32> = result.get(0).get(0);
                            filled[&k] = json!(result_val);
                        } else if ModuleConfig::_postgres_try_parse::<f32>(&row) {
                            let result_val: Option<f32> = result.get(0).get(0);
                            filled[&k] = json!(result_val);
                        } else if ModuleConfig::_postgres_try_parse::<f64>(&row) {
                            let result_val: Option<f64> = result.get(0).get(0);
                            filled[&k] = json!(result_val);
                        } else if ModuleConfig::_postgres_try_parse::<i64>(&row) {
                            let result_val: Option<i64> = result.get(0).get(0);
                            filled[&k] = json!(result_val);
                        } else if ModuleConfig::_postgres_try_parse::<bool>(&row) {
                            let result_val: Option<bool> = result.get(0).get(0);
                            filled[&k] = json!(result_val);
                        } else {
                            return Err(PafError::create_error(&format!("Invalid type found with query {}", query)));
                        }
                    } else {
                        return Err(PafError::create_error(&format!("Query ({}) did not return any rows.", query)));
                    }
                }
            }
        }
        self.params = Some(filled);
        Ok(())
    }

    fn _postgres_try_parse<T>(row: &Row) -> bool where T: FromSql {
        let test_type = panic::catch_unwind(|| {
            let _: Option<T> = row.get(0);
        });

        if test_type.is_ok() {
            return true;
        }
        false
    }

    fn _fill_with_mysql(&mut self) -> Result<(), Box<Error>> {
        let cstr = format!("mysql://{}", self.connection_string.as_ref().unwrap());
        let conn = mysql::Pool::new(cstr)?;
        let mut filled = self.as_map();

        for (k, v) in self.as_map() {
            if let Some(val) = v.as_str() {
                if let Some(info) = ModuleConfig::_read_db_string(val) {
                    let query = format!("SELECT {} FROM {} WHERE {} = {}", info[1], info[0], info[2], info[3]);
                    let result = conn.first_exec(query.to_string(), ())?;
                    if let Some(row) = result {
                        match &row.columns()[0].column_type() {
                            ColumnType::MYSQL_TYPE_STRING | ColumnType::MYSQL_TYPE_VARCHAR | ColumnType::MYSQL_TYPE_VAR_STRING =>
                                filled[&k] = json!(mysql::from_row::<Option<String>>(row)),
                            ColumnType::MYSQL_TYPE_INT24 | ColumnType::MYSQL_TYPE_LONG | ColumnType::MYSQL_TYPE_SHORT | ColumnType::MYSQL_TYPE_TINY =>
                                filled[&k] = json!(mysql::from_row::<Option<i64>>(row)),
                            ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_DOUBLE | ColumnType::MYSQL_TYPE_FLOAT =>
                                filled[&k] = json!(mysql::from_row::<Option<f64>>(row)),
                            _ => return Err(PafError::create_error(&format!("Invalid type found with query {}", query)))
                        }
                    } else {
                        return Err(PafError::create_error(&format!("Query ({}) did not return any rows.", query)));
                    }
                }
            }
        }

        self.params = Some(filled);
        Ok(())
    }

    fn _fill_with_sqlite(&mut self) -> Result<(), Box<Error>> {
        let con = sqlite::open(self.connection_string.as_ref().unwrap())?;
        let mut filled = self.as_map();

        for (k, v) in self.as_map() {
            if let Some(val) = v.as_str() {
                if let Some(info) = ModuleConfig::_read_db_string(val) {
                    let query = format!("SELECT {} FROM {} WHERE {} = {}", info[1], info[0], info[2], info[3]);
                    let mut result = con.prepare(query.to_string())?.cursor();
                    if let Some(row) = result.next()? {
                        match row[0].kind() {
                            sqlite::Type::String => filled[&k] = json!(row[0].as_string().unwrap()),
                            sqlite::Type::Integer => filled[&k] = json!(row[0].as_integer().unwrap()),
                            sqlite::Type::Float => filled[&k] = json!(row[0].as_float().unwrap()),
                            sqlite::Type::Null => filled[&k] = json!(null),
                            _ => return Err(PafError::create_error(&format!("Invalid type found with query {}", query)))
                        }
                    } else {
                        return Err(PafError::create_error(&format!("Query ({}) did not return any rows.", query)));
                    }
                }
            }
        }
        self.params = Some(filled);
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

    fn check_mysql_connection() -> bool {
        // In order to not fail MySQL tests, create a local server structure with the following parameters:
        // Database: openpaf
        // Username: openpaf_user (must have select privilege to the openpaf table)
        // Password: openpaf123
        // Port: 3306
        // Table: openpaf
        // Columns: id (int), param (varchar), number (int), nullable (varchar)
        // Add at least one row with VALUES (0, 'value', 12, NULL)
        let cstr = "mysql://openpaf_user:openpaf123@localhost:3306/openpaf";
        let conn_res = mysql::Pool::new(cstr);
        if conn_res.is_err() {
            panic!("Could not connect to MySQL Server.");
        }

        let conn = conn_res.unwrap();
        let query_res = conn.first_exec("SELECT * FROM openpaf WHERE id = 0", ());
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

    mod read_config {
        use super::super::*;

        #[test]
        fn works_without_db() {
            let conf = r#"{
                "params": {
                    "param1": "value1"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_ok());
        }

        #[test]
        fn with_db_needs_connection_string() {
            let conf = r#"{
                "db": "PostgreSQL",
                "params": {
                    "param1": "value1"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }

        #[test]
        fn fills_from_postgres() {
            let conf = r#"{
                "db": "PostgreSQL",
                "connection_string": "openpaf_user:openpaf123@localhost:5433/openpaf",
                "params": {
                    "param1": "db:openpaf/param/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_ok());
        }

        #[test]
        fn fills_from_sqlite() {
            let conf = r#"{
                "db": "SQLite",
                "connection_string": "test/openpaf_sqlite.db",
                "params": {
                    "param1": "db:openpaf/param/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_ok());
        }
    }

    mod _fill_with_postgres {
        use super::super::*;
        use super::*;

        #[test]
        fn check_connection() {
            assert!(check_postgres_connection())
        }

        #[test]
        fn reads_string() {
            let conf = r#"{
                "db": "PostgreSQL",
                "connection_string": "openpaf_user:openpaf123@localhost:5433/openpaf",
                "params": {
                    "param1": "db:openpaf/param/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], "value");
        }

        #[test]
        fn reads_number() {
            let conf = r#"{
                "db": "PostgreSQL",
                "connection_string": "openpaf_user:openpaf123@localhost:5433/openpaf",
                "params": {
                    "param1": "db:openpaf/numeric/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], 12);
        }

        #[test]
        fn reads_null() {
            let conf = r#"{
                "db": "PostgreSQL",
                "connection_string": "openpaf_user:openpaf123@localhost:5433/openpaf",
                "params": {
                    "param1": "db:openpaf/nullable/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], Value::Null);
        }

        #[test]
        fn throws_error_with_no_rows() {
            let conf = r#"{
                "db": "PostgreSQL",
                "connection_string": "openpaf_user:openpaf123@localhost:5433/openpaf",
                "params": {
                    "param1": "db:openpaf/nullable/id/9999"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }

        #[test]
        fn throws_error_with_bad_column() {
            let conf = r#"{
                "db": "PostgreSQL",
                "connection_string": "openpaf_user:openpaf123@localhost:5433/openpaf",
                "params": {
                    "param1": "db:openpaf/badcolumn/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }

        #[test]
        fn throws_error_with_bad_table() {
            let conf = r#"{
                "db": "PostgreSQL",
                "connection_string": "openpaf_user:openpaf123@localhost:5433/openpaf",
                "params": {
                    "param1": "db:badtable/nullable/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }
    }

    mod _fill_with_mysql {
        use super::super::*;
        use super::*;

        #[test]
        fn check_connection() {
            assert!(check_mysql_connection())
        }

        #[test]
        fn reads_string() {
            let conf = r#"{
                "db": "MySQL",
                "connection_string": "openpaf_user:openpaf123@localhost:3306/openpaf",
                "params": {
                    "param1": "db:openpaf/param/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], "value");
        }

        #[test]
        fn reads_number() {
            let conf = r#"{
                "db": "MySQL",
                "connection_string": "openpaf_user:openpaf123@localhost:3306/openpaf",
                "params": {
                    "param1": "db:openpaf/number/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], 12);
        }

        #[test]
        fn reads_null() {
            let conf = r#"{
                "db": "MySQL",
                "connection_string": "openpaf_user:openpaf123@localhost:3306/openpaf",
                "params": {
                    "param1": "db:openpaf/nullable/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], Value::Null);
        }

        #[test]
        fn throws_error_with_no_rows() {
            let conf = r#"{
                "db": "MySQL",
                "connection_string": "openpaf_user:openpaf123@localhost:3306/openpaf",
                "params": {
                    "param1": "db:openpaf/nullable/id/9999"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }

        #[test]
        fn throws_error_with_bad_column() {
            let conf = r#"{
                "db": "MySQL",
                "connection_string": "openpaf_user:openpaf123@localhost:3306/openpaf",
                "params": {
                    "param1": "db:openpaf/badcolumn/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }

        #[test]
        fn throws_error_with_bad_table() {
            let conf = r#"{
                "db": "MySQL",
                "connection_string": "openpaf_user:openpaf123@localhost:3306/openpaf",
                "params": {
                    "param1": "db:badtable/nullable/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }
    }

    mod _fill_with_sqlite {
        use super::super::*;

        #[test]
        fn reads_string() {
            let conf = r#"{
                "db": "SQLite",
                "connection_string": "test/openpaf_sqlite.db",
                "params": {
                    "param1": "db:openpaf/param/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], "value");
        }

        #[test]
        fn reads_number() {
            let conf = r#"{
                "db": "SQLite",
                "connection_string": "test/openpaf_sqlite.db",
                "params": {
                    "param1": "db:openpaf/numeric/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], 12);
        }

        #[test]
        fn reads_null() {
            let conf = r#"{
                "db": "SQLite",
                "connection_string": "test/openpaf_sqlite.db",
                "params": {
                    "param1": "db:openpaf/nullable/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf).unwrap();
            assert_eq!(modconf.as_map()["param1"], Value::Null);
        }

        #[test]
        fn throws_error_with_no_rows() {
            let conf = r#"{
                "db": "SQLite",
                "connection_string": "test/openpaf_sqlite.db",
                "params": {
                    "param1": "db:openpaf/nullable/id/9999"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }

        #[test]
        fn throws_error_with_bad_column() {
            let conf = r#"{
                "db": "SQLite",
                "connection_string": "test/openpaf_sqlite.db",
                "params": {
                    "param1": "db:openpaf/badcolumn/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }

        #[test]
        fn throws_error_with_bad_table() {
            let conf = r#"{
                "db": "SQLite",
                "connection_string": "test/openpaf_sqlite.db",
                "params": {
                    "param1": "db:badtable/nullable/id/0"
                }
            }"#;

            let modconf = ModuleConfig::read_config(conf);
            assert!(modconf.is_err());
        }
    }
}