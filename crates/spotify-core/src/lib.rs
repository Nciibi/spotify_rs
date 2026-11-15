//! Core of the spotify-rs desktop client: authentication, the Web API client
//! and the data models they exchange.

pub mod api;
pub mod auth;
pub mod models;

pub use api::{ApiClient, ApiError, TokenSet};
pub use models::*;

pub const APP_NAME: &str = "spotify-rs";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
