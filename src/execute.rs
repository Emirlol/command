use command::{
	Input,
	InputSource,
	error::DispatchError,
};

pub trait Executor<Ctx, Stack> {
	type Error: std::error::Error + Send + Sync;
	fn run(&self, ctx: &mut Ctx, stack: Stack) -> Result<(), Self::Error>;
}

/// A node that can dispatch commands based on the input.
pub trait ExecutableNode<Ctx, Stack, Source>
where
	Source: InputSource,
{
	/// Dispatches the command based on the input and the current context.
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>>;
}

internal_macros::impl_executor!(25); // Support stacks up to 25 deep -- this means there can be up to 25 consecutive argument nodes in a command. If this is not enough, you're doing something weird.
