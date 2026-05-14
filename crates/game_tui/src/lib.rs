//! FARSPACE Terminal User Interface
//!
//! This crate provides the terminal-based UI for FARSPACE.

pub mod app;
pub mod components;
pub mod faction;
pub mod keys;
pub mod layout;
pub mod map_render;
pub mod screens;
pub mod theme;
pub mod viewport;

pub use app::{App, AppState};
