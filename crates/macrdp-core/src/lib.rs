//! macrdp-core: unified RDP server library

pub mod callbacks;
pub mod config;
pub mod display;
pub mod handler;
pub mod permissions;
pub mod server;
pub mod tls;

pub use callbacks::*;
pub use config::{config_dir, ServerConfig};
pub use server::{start_server, ServerHandle};
