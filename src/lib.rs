#![allow(incomplete_features)]
#![feature(adt_const_params, unsized_const_params, min_specialization, const_default, const_trait_impl)]
//! Statically-typed command tree library.

extern crate self as command;

mod args;
mod error;
mod execute;
mod input;
mod inspect;
mod metadata;
mod node;

// Re-export for the macro to work without requiring the dependency on the consumer crates
#[cfg(feature = "const")]
pub mod const_default {
	pub use const_default::ConstDefault;
}

pub use args::*;
pub use error::*;
pub use execute::*;
pub use input::*;
pub use inspect::*;
pub use macros::command;
pub use metadata::*;
pub use node::*;
