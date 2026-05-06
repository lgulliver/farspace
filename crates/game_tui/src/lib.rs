//! FARSPACE Terminal User Interface
//!
//! This crate provides the terminal-based UI for FARSPACE.

pub mod app;
pub mod components;
pub mod keys;
pub mod layout;
pub mod screens;
pub mod theme;

pub use app::{App, AppState};
