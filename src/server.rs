extern crate machine_ip;

use std::error::Error;
use super::error::PafError;

/// Struct representing individual servers in a server chain.
pub struct Server {
    pub ip: String,
    pub ssh_port: u32
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

    /// Gets the next server in an unordered array of servers. Sorts the array, identifies
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
    ///     Server {ip: "172.16.5.251".to_string(), ssh_port: 22},
    ///     Server {ip: "172.16.5.250".to_string(), ssh_port: 22},
    ///     Server {ip: "172.11.3.110".to_string(), ssh_port: 22},
    ///     Server {ip: "172.13.1.121".to_string(), ssh_port: 22}
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
}

#[cfg(test)]
mod test {
    mod _sort {
        use super::super::*;

        #[test]
        fn sorts_servers() {
            let mut servers = vec![
                Server {ip: "172.16.5.251".to_string(), ssh_port: 22},
                Server {ip: "172.16.5.250".to_string(), ssh_port: 22},
                Server {ip: "172.11.3.110".to_string(), ssh_port: 22},
                Server {ip: "172.13.1.121".to_string(), ssh_port: 22}
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

    mod next_server {
        use super::super::*;

        #[test]
        fn identifies_current_ip() {
            let curr_ip = machine_ip::get().unwrap().to_string();
            let mut servers = vec![
                Server {ip: "172.16.5.251".to_string(), ssh_port: 22},
                Server {ip: "172.16.5.250".to_string(), ssh_port: 22},
                Server {ip: curr_ip, ssh_port: 22},
                Server {ip: "172.13.1.121".to_string(), ssh_port: 22}
            ];

            assert!(Server::next_server(&mut servers, None).is_ok())
        }

        #[test]
        fn errs_if_current_ip_not_in_list() {
            let mut servers = vec![
                Server {ip: "172.16.5.251".to_string(), ssh_port: 22},
                Server {ip: "172.16.5.250".to_string(), ssh_port: 22},
                Server {ip: "172.11.3.110".to_string(), ssh_port: 22},
                Server {ip: "172.13.1.121".to_string(), ssh_port: 22}
            ];

            assert!(Server::next_server(&mut servers, None).is_err())
        }

        #[test]
        fn accepts_optional_ip() {
            let mut servers = vec![
                Server {ip: "172.16.5.251".to_string(), ssh_port: 22},
                Server {ip: "172.16.5.250".to_string(), ssh_port: 22},
                Server {ip: "172.11.3.110".to_string(), ssh_port: 22},
                Server {ip: "172.13.1.121".to_string(), ssh_port: 22}
            ];

            assert!(Server::next_server(&mut servers, Some("172.16.5.250".to_string())).is_ok())
        }

        #[test]
        fn returns_correct_server() {
            let mut servers = vec![
                Server {ip: "172.16.5.251".to_string(), ssh_port: 22},
                Server {ip: "172.16.5.250".to_string(), ssh_port: 22},
                Server {ip: "172.11.3.110".to_string(), ssh_port: 22},
                Server {ip: "172.13.1.121".to_string(), ssh_port: 22}
            ];

            assert_eq!(Server::next_server(&mut servers, Some("172.16.5.250".to_string())).unwrap().ip, "172.16.5.251");
            assert_eq!(Server::next_server(&mut servers, Some("172.16.5.251".to_string())).unwrap().ip, "172.11.3.110");
        }
    }
}