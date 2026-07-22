#![allow(incomplete_features)]
#![feature(adt_const_params, unsized_const_params, min_specialization)]
//! Statically-typed command tree library.

mod args;
mod node;
mod error;

// Re-export for the macro to work without requiring the dependency on the consumer crates
#[cfg(feature = "const")]
pub mod const_default {
	pub use const_default::ConstDefault;
}

pub use args::{
	ArgParser,
	BoolParser,
	F32Parser,
	F64Parser,
	I16Parser,
	I32Parser,
	I64Parser,
	I8Parser,
	ParseError,
	StringParser,
	U16Parser,
	U32Parser,
	U64Parser,
	U8Parser,
};
pub use macros::command;
pub use node::{
	Argument,
	Choice,
	CommandNode,
	Exec,
	Executor,
	Input,
	Literal,
	LiteralAliases,
	Nil,
	Root,
	WithExec,
};
pub use error::CommandError;