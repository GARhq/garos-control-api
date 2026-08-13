//! Wake-on-LAN: send a magic packet over UDP broadcast.

use crate::config::WolSettings;
use crate::error::{AppError, IntegrationKind};
use async_trait::async_trait;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;
use uuid::Uuid;

#[async_trait]
pub trait Wol: Send + Sync {
    async fn send(&self, mac: &str) -> Result<WolReceipt, AppError>;
    async fn send_many(&self, macs: &[&str]) -> Result<Vec<WolReceipt>, AppError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct WolReceipt {
    pub mac: String,
    pub sent_at: chrono::DateTime<chrono::Utc>,
    pub broadcast: String,
    pub port: u16,
}

pub struct WolIntegration {
    settings: WolSettings,
    mock: bool,
}

impl WolIntegration {
    pub fn new(settings: WolSettings, mock: bool) -> Self {
        Self { settings, mock }
    }

    pub fn build_magic_packet(mac: &str) -> Result<Vec<u8>, AppError> {
        let bytes = parse_mac(mac)?;
        let mut pkt = Vec::with_capacity(6 + 16 * 6);
        pkt.extend_from_slice(&[0xFFu8; 6]);
        for _ in 0..16 {
            pkt.extend_from_slice(&bytes);
        }
        Ok(pkt)
    }
}

#[async_trait]
impl Wol for WolIntegration {
    async fn send(&self, mac: &str) -> Result<WolReceipt, AppError> {
        if self.mock {
            tracing::info!(target: "wol", %mac, "[mock] sent magic packet");
            return Ok(WolReceipt {
                mac: mac.into(),
                sent_at: chrono::Utc::now(),
                broadcast: self.settings.broadcast_addr.clone(),
                port: self.settings.port,
            });
        }
        let pkt = Self::build_magic_packet(mac)?;
        let sock = UdpSocket::bind(("0.0.0.0", 0))
            .await
            .map_err(|e| AppError::Integration {
                kind: IntegrationKind::Wol,
                message: format!("bind: {e}"),
            })?;
        sock.set_broadcast(true).map_err(|e| AppError::Integration {
            kind: IntegrationKind::Wol,
            message: format!("set_broadcast: {e}"),
        })?;
        let addr: SocketAddrV4 = format!("{}:{}", self.settings.broadcast_addr, self.settings.port)
            .parse()
            .map_err(|e: std::net::AddrParseError| AppError::Integration {
                kind: IntegrationKind::Wol,
                message: format!("bad broadcast addr: {e}"),
            })?;
        sock.send_to(&pkt, addr).await.map_err(|e| AppError::Integration {
            kind: IntegrationKind::Wol,
            message: format!("send: {e}"),
        })?;
        Ok(WolReceipt {
            mac: mac.into(),
            sent_at: chrono::Utc::now(),
            broadcast: self.settings.broadcast_addr.clone(),
            port: self.settings.port,
        })
    }

    async fn send_many(&self, macs: &[&str]) -> Result<Vec<WolReceipt>, AppError> {
        let mut out = Vec::with_capacity(macs.len());
        for mac in macs {
            out.push(self.send(mac).await?);
        }
        Ok(out)
    }
}

fn parse_mac(mac: &str) -> Result<[u8; 6], AppError> {
    let cleaned: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_lowercase();
    if cleaned.len() != 12 {
        return Err(AppError::BadRequest(format!("invalid MAC: {mac}")));
    }
    let mut out = [0u8; 6];
    for i in 0..6 {
        out[i] = u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16)
            .map_err(|e| AppError::BadRequest(format!("bad hex in MAC: {e}")))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn magic_packet_format() {
        let mac = "AA:BB:CC:DD:EE:FF";
        let pkt = WolIntegration::build_magic_packet(mac).unwrap();
        assert_eq!(pkt.len(), 6 + 16 * 6);
        assert_eq!(&pkt[..6], &[0xFFu8; 6]);
        for i in 0..16 {
            assert_eq!(&pkt[6 + i * 6..6 + (i + 1) * 6], &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        }
    }

    #[test]
    fn rejects_garbage_mac() {
        assert!(WolIntegration::build_magic_packet("not-a-mac").is_err());
    }

    #[tokio::test]
    async fn mock_send() {
        let w = WolIntegration::new(WolSettings::default(), true);
        let r = w.send("AA:BB:CC:DD:EE:FF").await.unwrap();
        assert_eq!(r.mac, "AA:BB:CC:DD:EE:FF");
    }
}
