#![allow(incomplete_features)]
#![feature(
	adt_const_params,
	associated_type_defaults,
	const_convert,
	const_default,
	const_index,
	const_iter,
	const_option_ops,
	const_trait_impl,
	iter_intersperse,
	min_specialization,
	unsized_const_params
)]
#![cfg_attr(not(feature = "async-suggestions"), feature(const_path_separators))]

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
mod suggest;
pub use suggest::*;
