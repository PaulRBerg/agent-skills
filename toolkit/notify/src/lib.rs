#![deny(unsafe_code)]

pub mod atomic_write;
pub mod cli;
pub mod config;
pub mod error;
pub mod events;
pub mod filters;
pub mod integrations;
pub mod logging;
pub mod model;
pub mod notifier;
pub mod state;

pub use error::{AppError, ErrorKind, Result};
