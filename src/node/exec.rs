use crate::{CommandError, Executor, Input, InputSource};
use crate::error::{DispatchError, IntoDispatchError};
use crate::node::dispatch_node::DispatchNode;

/// A terminal node that executes a function.
pub struct Exec<F> {
	executor: F,
}

impl<F> Exec<F> {
	pub const fn new(executor: F) -> Self {
		Self { executor }
	}
}

impl<F: Default> Default for Exec<F> {
	fn default() -> Self {
		Self::new(F::default())
	}
}

impl<Ctx, Stack, F, Source> DispatchNode<Ctx, Stack, Source> for Exec<F>
where
	Source: InputSource,
	F: Executor<Ctx, Stack>,
	F::Error: IntoDispatchError,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		if let Some(tokens) = input.remaining_tokens() {
			return Err(DispatchError::Fatal(CommandError::TrailingInput(tokens.join(" "))));
		}

		self.executor.run(ctx, stack).map_err(IntoDispatchError::into_fatal) // IntoDispatchError has a specialization for CommandError to not box it again
	}
}