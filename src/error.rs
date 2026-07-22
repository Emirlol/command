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

pub(crate) enum DispatchError<Stack> {
	Recoverable(CommandError, Stack),
	Fatal(CommandError),
}

pub(crate) trait IntoDispatchError {
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
