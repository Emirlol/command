#![feature(adt_const_params)]
#![allow(incomplete_features)]
#![feature(unsized_const_params)]

//! Static typed command tree

mod args;
mod node;

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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CommandError {
	#[error("unknown command '{0}'")]
	UnknownCommand(String),
	#[error("unknown argument '{0}'")]
	UnknownArgument(String),
	#[error("invalid argument '{name}' with value '{value}': {reason}")]
	InvalidArgument { name: &'static str, value: String, reason: String },
	#[error("incomplete command")]
	IncompleteCommand,
	#[error("trailing input '{0}'")]
	TrailingInput(String),
	#[error("command execution failed: {0}")]
	Execution(String),
}
