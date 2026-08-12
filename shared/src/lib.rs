#![allow(clippy::unsafe_derive_deserialize)]

mod app;
pub mod ffi;
mod reminder;

pub use app::*;
pub use reminder::*;
pub use crux_core::Core;
