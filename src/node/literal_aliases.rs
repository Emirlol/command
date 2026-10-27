use crate::{CommandError, Input, InputSource};
use crate::error::DispatchError;
use crate::node::dispatch_node::DispatchNode;

/// A literal node that can have match multiple string literals.
/// 
/// The main use case is to not replicate children nodes in the command tree by creating a graph that consists of 1-n-1 layers instead of 1-n-n. 
pub struct LiteralAliases<const NAMES: &'static [&'static str], Children> {
	children: Children,
}

impl<const NAMES: &'static [&'static str], Children> LiteralAliases<NAMES, Children> {
	pub const fn new(children: Children) -> Self {
		Self { children }
	}
}

impl<const NAMES: &'static [&'static str], Children: Default> Default for LiteralAliases<NAMES, Children> {
	fn default() -> Self {
		Self::new(Children::default())
	}
}

impl<Ctx, Stack, const NAMES: &'static [&'static str], Children, Source> DispatchNode<Ctx, Stack, Source> for LiteralAliases<NAMES, Children>
where
	Source: InputSource,
	Children: DispatchNode<Ctx, Stack, Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();

		match input.next_token() {
			Some(token) if NAMES.iter().any(|name| token == *name) => self.children.dispatch(ctx, input, stack).map_err(DispatchError::into_fatal),
			Some(token) => {
				let token = token.to_owned(); // Drop the mutable reference to input to allow restoring the checkpoint, otherwise it would be a double mutable borrow of input.
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(CommandError::UnknownCommand(token), stack))
			}
			None => Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack)),
		}
	}
}