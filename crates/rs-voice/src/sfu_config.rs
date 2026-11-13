use rs_common::Config;

/// Network and ICE configuration for the WebRTC SFU.
#[derive(Debug, Clone)]
pub struct SfuConfig {
    /// Local IP address to bind ICE UDP sockets.
    pub bind_ip: String,
    /// Optional STUN server URL (e.g. `stun:stun.l.google.com:19302`).
    pub stun_url: Option<String>,
    /// Inclusive lower bound of the UDP port range.
    pub udp_port_min: u16,
    /// Inclusive upper bound of the UDP port range.
    pub udp_port_max: u16,
}

impl From<&Config> for SfuConfig {
    fn from(cfg: &Config) -> Self {
        Self {
            bind_ip: cfg.ice_bind_ip.clone(),
            stun_url: cfg.stun_url.clone(),
            udp_port_min: cfg.ice_udp_port_min,
            udp_port_max: cfg.ice_udp_port_max,
        }
    }
}
