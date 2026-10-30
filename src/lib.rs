#![allow(incomplete_features)]
#![feature(
	adt_const_params,
	unsized_const_params,
	min_specialization,
	const_default,
	const_trait_impl,
	const_convert,
	const_iter,
	const_option_ops,
	const_index,
	iter_intersperse
)]
//! Statically-typed command tree library.

extern crate self as command;

mod args;
pub use args::*;
mod error;
pub use error::*;
mod execute;
pub use execute::*;
mod input;

pub use input::*;
mod inspect;
pub use inspect::*;
pub use macros::command;
mod metadata;
pub use metadata::*;
mod node;
pub use node::*;
