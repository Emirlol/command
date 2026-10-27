use crate::{CommandError, Input, InputSource};
use crate::error::DispatchError;
use crate::node::dispatch_node::DispatchNode;

/// A literal node, matched by a single string literal.
pub struct Literal<const NAME: &'static str, Children> {
	children: Children,
}

impl<const NAME: &'static str, Children> Literal<NAME, Children> {
	pub const fn new(children: Children) -> Self {
		Self { children }
	}
}

impl<const NAME: &'static str, Children: Default> Default for Literal<NAME, Children> {
	fn default() -> Self {
		Self::new(Children::default())
	}
}

impl<Ctx, Stack, const NAME: &'static str, Children, Source> DispatchNode<Ctx, Stack, Source> for Literal<NAME, Children>
where
	Source: InputSource,
	Children: DispatchNode<Ctx, Stack, Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		match input.next_token() {
			Some(token) if token == NAME => self.children.dispatch(ctx, input, stack).map_err(DispatchError::into_fatal),
			Some(token) => {
				let token = token.to_owned(); // Drop the mutable reference to input to allow restoring the checkpoint, otherwise it would be a double mutable borrow of input.
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(CommandError::UnknownCommand(token), stack))
			}
			None => Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack)),
		}
	}
}
