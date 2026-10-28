use std::ffi::OsString;

use crate::{
	Input,
	InputSource,
};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
	// Internal errors
	#[error("Unknown command '{0}'")]
	UnknownCommand(String),
	#[error("Invalid argument value '{value}' for arg '{name}': {reason}")]
	InvalidArgument { name: &'static str, value: String, reason: String },
	#[error("Incomplete command")]
	IncompleteCommand,
	#[error("Unexpected trailing input: '{0}'")]
	TrailingInput(String),
	#[error(transparent)]
	InputError(#[from] InputError),

	// Meant for external use, this error can be returned instead of other types as a generic message return type.
	#[error("Command execution failed: {0}")]
	Execution(String),
	// As an alternative to `Execution`, this error can be used to wrap any external error type.
	#[error("Unknown error: {0:?}")]
	External(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// An alternative to `anyhow`'s `bail!` macro for use within command executors that uses the `CommandError` type for easier handling.
#[macro_export]
#[clippy::format_args]
macro_rules! command_error {
    ($($arg:tt)*) => {
        return Err($crate::CommandError::Execution(format!($($arg)*)))
    };
}

/// More lightweight errors that aren't fully initialized `CommandError`s, but can be converted into them with the help of the input source.
#[derive(Debug)]
pub enum RecoverableError {
	UnknownCommand { offset: usize },
	InvalidArgument { name: &'static str, offset: usize, error: String },
	IncompleteCommand,
	TrailingInput { offset: usize },
}

impl RecoverableError {
	pub fn into_command_error<Source>(self, input: &Input<Source>) -> CommandError
	where
		Source: InputSource,
	{
		match self {
			RecoverableError::UnknownCommand { offset } => {
				let Some(token) = input.get(offset) else {
					unreachable!(
						"Offset {} is out of bounds for input `{}`",
						offset,
						input.remaining_tokens_at(offset).map(|tokens| tokens.join(" ")).unwrap_or_default()
					);
				};
				CommandError::UnknownCommand(token.to_string())
			}
			RecoverableError::InvalidArgument { name, offset, error } => {
				let Some(token) = input.get(offset) else {
					unreachable!(
						"Offset {} is out of bounds for input `{}`",
						offset,
						input.remaining_tokens_at(offset).map(|tokens| tokens.join(" ")).unwrap_or_default()
					);
				};
				CommandError::InvalidArgument {
					name,
					value: token.to_string(),
					reason: error.to_string(),
				}
			}
			RecoverableError::IncompleteCommand => CommandError::IncompleteCommand,
			RecoverableError::TrailingInput { offset } => {
				let Some(token) = input.get(offset) else {
					unreachable!(
						"Offset {} is out of bounds for input `{}`",
						offset,
						input.remaining_tokens_at(offset).map(|tokens| tokens.join(" ")).unwrap_or_default()
					);
				};
				CommandError::TrailingInput(token.to_string())
			}
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum InputError {
	#[error("Invalid UTF-8 input: {0:?}")]
	InvalidUtf8(OsString),
}

#[derive(Debug)]
pub enum DispatchError<Stack> {
	Recoverable(RecoverableError, Stack),
	Fatal(CommandError),
}

impl<Stack> DispatchError<Stack> {
	pub fn into_fatal<Source>(self, input: &Input<Source>) -> DispatchError<Stack>
	where
		Source: InputSource,
	{
		match self {
			DispatchError::Recoverable(error, _) => DispatchError::Fatal(error.into_command_error(input)),
			DispatchError::Fatal(error) => DispatchError::Fatal(error),
		}
	}

	pub fn into_fatal_for_parent<Source, ParentStack>(self, input: &Input<Source>) -> DispatchError<ParentStack>
	where
		Source: InputSource,
	{
		match self {
			DispatchError::Recoverable(error, _) => DispatchError::Fatal(error.into_command_error(input)),
			DispatchError::Fatal(error) => DispatchError::Fatal(error),
		}
	}
}

pub trait IntoDispatchError {
	fn into_fatal<Stack>(self) -> DispatchError<Stack>;
}

impl<E> IntoDispatchError for E
where
	E: std::error::Error + Send + Sync + 'static,
{
	#[inline(always)]
	default fn into_fatal<Stack>(self) -> DispatchError<Stack> {
		DispatchError::Fatal(CommandError::External(Box::new(self)))
	}
}

impl IntoDispatchError for CommandError {
	#[inline(always)]
	fn into_fatal<Stack>(self) -> DispatchError<Stack> {
		DispatchError::Fatal(self)
	}
}
