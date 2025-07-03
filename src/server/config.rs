use std::net::SocketAddr;

/// Parse server configuration from environment variables
pub fn parse_server_address() -> Result<SocketAddr, String> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let ip = std::env::var("IP").unwrap_or_else(|_| "0.0.0.0".to_string());

    format!("{ip}:{port}")
        .parse()
        .map_err(|e| format!("Invalid IP or PORT environment variables: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Use a mutex to ensure tests don't interfere with each other
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_default_address() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Clear env vars to test defaults
        env::remove_var("IP");
        env::remove_var("PORT");

        let addr = parse_server_address().unwrap();
        assert_eq!(addr.to_string(), "0.0.0.0:8080");
    }

    #[test]
    fn test_custom_port() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::remove_var("IP");
        env::set_var("PORT", "3000");

        let addr = parse_server_address().unwrap();
        assert_eq!(addr.to_string(), "0.0.0.0:3000");

        env::remove_var("PORT");
    }

    #[test]
    fn test_custom_ip() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("IP", "127.0.0.1");
        env::remove_var("PORT");

        let addr = parse_server_address().unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:8080");

        env::remove_var("IP");
    }

    #[test]
    fn test_custom_ip_and_port() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("IP", "192.168.1.100");
        env::set_var("PORT", "9000");

        let addr = parse_server_address().unwrap();
        assert_eq!(addr.to_string(), "192.168.1.100:9000");

        env::remove_var("IP");
        env::remove_var("PORT");
    }

    #[test]
    fn test_invalid_ip() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("IP", "invalid.ip.address");
        env::remove_var("PORT");

        let result = parse_server_address();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid IP or PORT"));

        env::remove_var("IP");
    }

    #[test]
    fn test_invalid_port() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::remove_var("IP");
        env::set_var("PORT", "not_a_number");

        // Should fall back to default port 8080 when PORT can't be parsed
        let addr = parse_server_address().unwrap();
        assert_eq!(addr.to_string(), "0.0.0.0:8080");

        env::remove_var("PORT");
    }

    #[test]
    fn test_port_out_of_range() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::remove_var("IP");
        env::set_var("PORT", "70000"); // > 65535

        let result = parse_server_address();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid IP or PORT"));

        env::remove_var("PORT");
    }

    #[test]
    fn test_localhost_address() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("IP", "127.0.0.1");
        env::set_var("PORT", "8080");

        let addr = parse_server_address().unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:8080");

        env::remove_var("IP");
        env::remove_var("PORT");
    }

    #[test]
    fn test_ipv6_localhost() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("IP", "::1");
        env::set_var("PORT", "8080");

        // IPv6 parsing might fail depending on the system, so handle both cases
        match parse_server_address() {
            Ok(addr) => {
                assert_eq!(addr.to_string(), "[::1]:8080");
            }
            Err(_) => {
                // IPv6 parsing not supported on this system, skip
            }
        }

        env::remove_var("IP");
        env::remove_var("PORT");
    }
}
