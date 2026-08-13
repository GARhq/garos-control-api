//! # garos-backend
//!
//! Production HTTP API for the kryonix-os-control-center to manage diskless
//! garos endpoints on NixOS.
//!
//! This crate exposes the library API used by the `garos-backend` binary in
//! [`main.rs`](main.rs) and integration tests. It is split into:
//!
//! - [`config`] — typed configuration loaded from `config/*.toml` and env
//! - [`error`] — `AppError` enum and HTTP response mapping
//! - [`telemetry`] — `tracing` + optional OpenTelemetry init
//! - [`state`] — shared `AppState`
//! - [`auth`] — JWT + Argon2id + axum extractors and middleware
//! - [`db`] — sqlx pool, models, repositories
//! - [`domain`] — DTOs and validators
//! - [`handlers`] — axum handlers
//! - [`integrations`] — Nix, Samba, BTRFS, NFTables, systemd, WOL, PXE, journald
//! - [`middleware`] — request_id, logging, rate limiting, CORS
//! - [`realtime`] — WebSocket pub/sub hub
//! - [`services`] — business logic
//! - [`api`] — router + OpenAPI

#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::struct_excessive_bools,
    clippy::option_if_let_else
)]

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod handlers;
pub mod integrations;
pub mod metrics;
pub mod middleware;
pub mod realtime;
pub mod services;
pub mod state;
pub mod telemetry;

pub use config::Settings;
pub use error::{AppError, AppResult};
pub use state::AppState;
