use crate::{
	Input,
	InputSource,
	error::DispatchError,
	node::dispatch_node::DispatchNode,
};

/// A node that has 2 child nodes.
///
/// This is the main building block of a command tree, allowing for branching between different command paths.
pub struct Choice<Head, Tail> {
	head: Head,
	tail: Tail,
}

impl<Head, Tail> Choice<Head, Tail> {
	pub const fn new(head: Head, tail: Tail) -> Self {
		Self { head, tail }
	}
}

impl<Head: Default, Tail: Default> Default for Choice<Head, Tail> {
	fn default() -> Self {
		Self::new(Head::default(), Tail::default())
	}
}

impl<Ctx, Stack, Head, Tail, Source> DispatchNode<Ctx, Stack, Source> for Choice<Head, Tail>
where
	Source: InputSource,
	Head: DispatchNode<Ctx, Stack, Source>,
	Tail: DispatchNode<Ctx, Stack, Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		match self.head.dispatch(ctx, input, stack) {
			Ok(()) => Ok(()),
			Err(DispatchError::Fatal(error)) => Err(DispatchError::Fatal(error)),
			Err(DispatchError::Recoverable(_, stack)) => {
				input.restore(checkpoint);
				self.tail.dispatch(ctx, input, stack)
			}
		}
	}
}
