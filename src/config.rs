use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub rtsp_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".into())
            .parse()
            .expect("invalid BIND_ADDR");

        let rtsp_url = std::env::var("RTSP_URL").unwrap_or_else(|_| {
            "rtsp://admin:abc303306@192.168.211.8:554/cam/realmonitor?channel=1&subtype=1".into()
        });

        Self {
            bind_addr,
            rtsp_url,
        }
    }
}
