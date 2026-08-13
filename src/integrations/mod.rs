//! Wrappers for system integrations (Nix, Samba, BTRFS, NFTables, systemd, WOL, PXE, journald).
//!
//! Every integration has a `Mock` implementation (returned when
//! `features.mock_integrations = true`) and a real implementation that
//! spawns subprocesses via `tokio::process::Command` and parses their
//! output into typed structs.

pub mod btrfs;
pub mod journald;
pub mod nftables;
pub mod nix;
pub mod pxe;
pub mod samba;
pub mod systemd;
pub mod wol;

pub use btrfs::BtrfsIntegration;
pub use journald::JournaldIntegration;
pub use nftables::NftablesIntegration;
pub use nix::NixIntegration;
pub use pxe::PxeIntegration;
pub use samba::SambaIntegration;
pub use systemd::SystemdIntegration;
pub use wol::WolIntegration;
