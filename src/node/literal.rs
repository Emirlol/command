use crate::{
	DispatchNode,
	Input,
	InputSource,
	error::{
		DispatchError,
		RecoverableError,
	},
};

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
		match input.peek() {
			Some(token) if token == NAME => {
				input.advance(1);
				self.children.dispatch(ctx, input, stack).map_err(|e| match e {
					DispatchError::Recoverable(error, _) => DispatchError::Fatal(error.into_command_error(input)),
					DispatchError::Fatal(error) => DispatchError::Fatal(error),
				})
			}
			Some(_) => Err(DispatchError::Recoverable(RecoverableError::UnknownCommand { offset: input.offset }, stack)),
			None => Err(DispatchError::Recoverable(RecoverableError::IncompleteCommand, stack)),
		}
	}
}
