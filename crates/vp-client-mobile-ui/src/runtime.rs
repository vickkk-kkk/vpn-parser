//! Platform runtime interface. No implementation or side effect is provided here.
use std::net::SocketAddr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError { Unavailable, Unsupported, InvalidInput, PermissionRequired, Failed }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeObservation { Unavailable, Idle, Starting, Verifying, ReportedActive, Stopping, Failed, Stale }

/// Implementations own and erase their staged secrets, and must preflight the
/// exact configuration before consent and before starting. No implicit fallback.
pub trait RuntimeHost {
    fn available(&mut self) -> bool;
    fn observe(&mut self) -> RuntimeObservation;
    fn prepare(&mut self, raw_uri: &str, node_id: &str, bootstrap: &[SocketAddr], user_dns: &[SocketAddr]) -> Result<(), RuntimeError>;
    fn request_consent(&mut self) -> Result<(), RuntimeError>;
    fn consent_granted(&mut self) -> bool;
    fn start(&mut self) -> Result<(), RuntimeError>;
    fn stop(&mut self) -> Result<(), RuntimeError>;
    fn clear_prepared(&mut self);
}

/// Numeric IP:port entries only, with no implicit DNS defaults.
pub fn parse_dns(input: &str) -> Result<Vec<SocketAddr>, RuntimeError> {
    let mut out = Vec::new();
    for line in input.lines().map(str::trim).filter(|s| !s.is_empty()) {
        let addr: SocketAddr = line.parse().map_err(|_| RuntimeError::InvalidInput)?;
        if addr.port() == 0 || addr.ip().is_unspecified() || addr.ip().is_multicast() || out.len() == 8 {
            return Err(RuntimeError::InvalidInput);
        }
        out.push(addr);
    }
    if out.is_empty() { Err(RuntimeError::InvalidInput) } else { Ok(out) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn explicit_numeric_dns_only() {
        assert_eq!(parse_dns("1.1.1.1:53\n[2001:db8::1]:53").unwrap().len(), 2);
        for input in ["", "resolver.example:53", "0.0.0.0:53", "224.0.0.1:53", "1.1.1.1:0"] {
            assert_eq!(parse_dns(input), Err(RuntimeError::InvalidInput));
        }
        assert_eq!(parse_dns(&"1.1.1.1:53\n".repeat(9)), Err(RuntimeError::InvalidInput));
    }
}
