use crate::{
	CommandError,
	Input,
	InputSource,
};
use crate::error::DispatchError;
use crate::node::dispatch_node::DispatchNode;

/// A node in the command tree that can be executed.
pub trait CommandNode<Ctx, Stack, Source>
where
	Source: InputSource,
{
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), CommandError>;
}


impl<Ctx, Stack, Node, Source> CommandNode<Ctx, Stack, Source> for Node
where
	Source: InputSource,
	Node: DispatchNode<Ctx, Stack, Source>,
{
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), CommandError> {
		match self.dispatch(ctx, input, stack) {
			Ok(()) => Ok(()),
			Err(DispatchError::Recoverable(error, _) | DispatchError::Fatal(error)) => Err(error),
		}
	}
}