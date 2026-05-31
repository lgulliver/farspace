//! FARSPACE Terminal User Interface
//!
//! This crate provides the terminal-based UI for FARSPACE.

pub mod animation;
pub mod app;
pub mod components;
#[cfg(any(test, feature = "e2e"))]
pub mod e2e_support;
pub mod faction;
pub mod glyphs;
pub mod keys;
pub mod layout;
pub mod map_render;
pub mod renderer;
pub mod screens;
pub mod theme;
pub mod update;
pub mod viewport;
pub mod visual_mode;

pub use app::{App, AppState};
pub use update::{UpdateChannel, UpdateInfo, UpdateState};
