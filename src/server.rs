use std::error::Error;
use serde::{Deserialize, Serialize};
use machine_ip;
use super::error::PafError;

/// Struct representing individual servers in a server chain.
#[derive(Deserialize, Serialize, Clone)]
pub struct Server {
    name: Option<String>,
    ip: String,
    ssh_port: Option<u32>
}

impl Server {
    /// Sorts a list of server objects in place.
    /// 
    /// ## Arguments
    /// * `servers` - array of servers
    fn _sort(servers: &mut Vec<Server>) {
        servers.sort_by(|a, b| a.ip.cmp(&b.ip))
    }

    /// Gets the current machine's IP, if no argument is provided.
    /// Returns the argument otherwise.
    /// 
    /// ## Arguments
    /// * `ip` - an optional IP
    fn _get_ip(ip: Option<String>) -> Option<String> {
        if let Some(input) = ip {
            Some(input)
        } else {
            if let Some(curr_ip) = machine_ip::get() {
                Some(curr_ip.to_string())
            } else {
                None
            }
        }
    }

    /// Constructor for the `Server` struct. Creates a new server object.
    pub fn new(name: Option<String>, ip: String, ssh_port: Option<u32>) -> Server {
        Server {
            name: name,
            ip: ip,
            ssh_port: ssh_port
        }
    }

    /// Finds the next server in an unordered array of servers. Sorts the array, identifies
    /// the provided IP (or the current machine's IP), and returns the next `Server` in the list.
    /// Returns an error, if the server cannot be found in the list.
    /// 
    /// ## Arguments
    /// * `servers` - array of servers
    /// * `ip` - an optional IP string
    /// 
    /// ## Examples
    /// ```
    /// let mut servers = vec![
    ///     Server {ip: "172.16.5.251".to_string(), ssh_port: None, name: None},
    ///     Server {ip: "172.16.5.250".to_string(), ssh_port: None, name: None},
    ///     Server {ip: "172.11.3.110".to_string(), ssh_port: None, name: None},
    ///     Server {ip: "172.13.1.121".to_string(), ssh_port: None, name: None}
    /// ];
    /// let next = Server::next_server(&mut servers, Some("172.16.5.250".to_string())).unwrap();
    /// assert_eq!(next.ip, "172.16.5.251");
    /// ```
    pub fn next_server(servers: &mut Vec<Server>, ip: Option<String>) -> Result<&Server, Box<Error>> {
        Server::_sort(servers);
        if let Some(needle) = Server::_get_ip(ip) {
            if let Some(i) = servers.iter().position(|e| e.ip == needle) {
                if i == servers.len() - 1 {
                    Ok(&servers[0])
                } else {
                    Ok(&servers[i + 1])
                }
            } else {
                Err(PafError::create_error("Could not find current machine's IP in the server list."))
            }
        } else {
            Err(PafError::create_error("Unable to extract current machine's IP."))
        }
    }

    /// Removes duplicate entries from a server list. Sorts the servers
    /// beforehand, therefore does not preserve order.
    /// 
    /// ## Arguments
    /// * `servers` - list of servers
    /// 
    /// ## Examples
    /// ```
    /// let mut servers = vec![
    ///     Server {ip: "172.16.5.251".to_string(), ssh_port: None, name: None},
    ///     Server {ip: "172.13.1.121".to_string(), ssh_port: None, name: None},
    ///     Server {ip: "172.11.3.110".to_string(), ssh_port: None, name: None},
    ///     Server {ip: "172.13.1.121".to_string(), ssh_port: None, name: None}
    /// ];
    /// Server::remove_duplicates(&mut servers);
    /// assert_eq!(servers.len(), 3);
    /// ```
    pub fn remove_duplicates(servers: &mut Vec<Server>) {
        Server::_sort(servers);
        servers.dedup_by(|a, b| a.ip == b.ip);
    }

    /// Returns the name of the server. If there is none,
    /// returns an empty string.
    pub fn name(&self) -> String {
        if let Some(name) = &self.name {
            name.to_string()
        } else {
            "".to_string()
        }
    }

    /// Returns the IP of the server.
    pub fn ip(&self) -> String {
        self.ip.to_string()
    }

    /// Returns the SSH port of the server. If there is none,
    /// returns the default port 22.
    pub fn ssh_port(&self) -> u32 {
        self.ssh_port.unwrap_or(22)
    }
}

#[cfg(test)]
mod test {
    mod _sort {
        use super::super::*;

        #[test]
        fn sorts_servers() {
            let mut servers = vec![
                Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None},
                Server {name: None, ip: "172.16.5.250".to_string(), ssh_port: None},
                Server {name: None, ip: "172.11.3.110".to_string(), ssh_port: None},
                Server {name: None, ip: "172.13.1.121".to_string(), ssh_port: None}
            ];
            Server::_sort(&mut servers);

            assert_eq!(servers[0].ip, "172.11.3.110");
            assert_eq!(servers[1].ip, "172.13.1.121");
            assert_eq!(servers[2].ip, "172.16.5.250");
            assert_eq!(servers[3].ip, "172.16.5.251");
        } 
    }

    mod _get_ip {
        use super::super::*;

        #[test]
        fn returns_arg_if_any() {
            assert_eq!(Server::_get_ip(Some("172.16.1.1".to_string())).unwrap(), "172.16.1.1");
        }

        #[test]
        fn returns_current_ip_if_no_arg() {
            let curr_ip = machine_ip::get().unwrap().to_string();
            assert_eq!(Server::_get_ip(None).unwrap(), curr_ip);
        }
    }

    mod new {
        use super::super::*;

        #[test]
        fn creates_new() {
            let server = Server::new(Some("name".to_string()), "172.11.3.110".to_string(), Some(2000));

            assert_eq!(server.name.unwrap(), "name".to_string());
            assert_eq!(server.ip, "172.11.3.110".to_string());
            assert_eq!(server.ssh_port.unwrap(), 2000);
        }
    }

    mod next_server {
        use super::super::*;

        #[test]
        fn identifies_current_ip() {
            let curr_ip = machine_ip::get().unwrap().to_string();
            let mut servers = vec![
                Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None},
                Server {name: None, ip: "172.16.5.250".to_string(), ssh_port: None},
                Server {name: None, ip: curr_ip, ssh_port: None},
                Server {name: None, ip: "172.13.1.121".to_string(), ssh_port: None}
            ];

            assert!(Server::next_server(&mut servers, None).is_ok())
        }

        #[test]
        fn errs_if_current_ip_not_in_list() {
            let mut servers = vec![
                Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None},
                Server {name: None, ip: "172.16.5.250".to_string(), ssh_port: None},
                Server {name: None, ip: "172.11.3.110".to_string(), ssh_port: None},
                Server {name: None, ip: "172.13.1.121".to_string(), ssh_port: None}
            ];

            assert!(Server::next_server(&mut servers, None).is_err())
        }

        #[test]
        fn accepts_optional_ip() {
            let mut servers = vec![
                Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None},
                Server {name: None, ip: "172.16.5.250".to_string(), ssh_port: None},
                Server {name: None, ip: "172.11.3.110".to_string(), ssh_port: None},
                Server {name: None, ip: "172.13.1.121".to_string(), ssh_port: None}
            ];

            assert!(Server::next_server(&mut servers, Some("172.16.5.250".to_string())).is_ok())
        }

        #[test]
        fn returns_correct_server() {
            let mut servers = vec![
                Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None},
                Server {name: None, ip: "172.16.5.250".to_string(), ssh_port: None},
                Server {name: None, ip: "172.11.3.110".to_string(), ssh_port: None},
                Server {name: None, ip: "172.13.1.121".to_string(), ssh_port: None}
            ];

            assert_eq!(Server::next_server(&mut servers, Some("172.16.5.250".to_string())).unwrap().ip, "172.16.5.251");
            assert_eq!(Server::next_server(&mut servers, Some("172.16.5.251".to_string())).unwrap().ip, "172.11.3.110");
        }
    }

    mod remove_duplicates {
        use super::super::*;

        #[test]
        fn removes_duplicates() {
            let mut servers = vec![
                Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None},
                Server {name: None, ip: "172.13.1.121".to_string(), ssh_port: None},
                Server {name: None, ip: "172.11.3.110".to_string(), ssh_port: None},
                Server {name: None, ip: "172.13.1.121".to_string(), ssh_port: None}
            ];
            Server::remove_duplicates(&mut servers);

            assert_eq!(servers.len(), 3);
        }

        #[test]
        fn works_with_empty() {
            let mut servers = vec![];
            Server::remove_duplicates(&mut servers);

            assert_eq!(servers.len(), 0);
        }
    }

    mod name {
        use super::super::*;

        #[test]
        fn returns_name_or_empty_string() {
            let server = Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None};
            let named_server = Server {name: Some("me".to_string()), ip: "172.16.5.251".to_string(), ssh_port: None};

            assert_eq!(server.name(), "".to_string());
            assert_eq!(named_server.name(), "me".to_string());
        }
    }

    mod ip {
        use super::super::*;

        #[test]
        fn returns_ip() {
            let server = Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None};

            assert_eq!(server.ip(), "172.16.5.251".to_string());
        }
    }

    mod ssh_port {
        use super::super::*;

        #[test]
        fn returns_port_or_default() {
            let server = Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: None};
            let server_w_port = Server {name: None, ip: "172.16.5.251".to_string(), ssh_port: Some(3000)};

            assert_eq!(server.ssh_port(), 22);
            assert_eq!(server_w_port.ssh_port(), 3000);
        }
    }
}