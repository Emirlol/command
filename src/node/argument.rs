use std::fmt::Display;
use crate::{ArgParser, CommandError, Input, InputSource};
use crate::error::DispatchError;
use crate::node::dispatch_node::DispatchNode;

pub struct Argument<const NAME: &'static str, Parser, Children> {
	parser: Parser,
	children: Children,
}

impl<const NAME: &'static str, Parser, Children> Argument<NAME, Parser, Children> {
	pub const fn new(parser: Parser, children: Children) -> Self {
		Self { parser, children }
	}
}

impl<const NAME: &'static str, Parser: Default, Children: Default> Default for Argument<NAME, Parser, Children> {
	fn default() -> Self {
		Self::new(Parser::default(), Children::default())
	}
}

impl<Ctx, Stack, const NAME: &'static str, Parser, Children, Source> DispatchNode<Ctx, Stack, Source> for Argument<NAME, Parser, Children>
where
	Source: InputSource,
	Parser: ArgParser,
	Parser::Error: Display,
	Children: DispatchNode<Ctx, (Parser::Output, Stack), Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		let Some(token) = input.next_token() else {
			return Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack));
		};

		match self.parser.parse(token) {
			Ok(value) => self.children.dispatch(ctx, input, (value, stack)).map_err(DispatchError::into_fatal_for_parent),
			Err(error) => {
				// Drop the mutable reference to input to allow restoring the checkpoint, otherwise it would be a double mutable borrow of input.
				let token = token.to_owned();
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(
					CommandError::InvalidArgument {
						name: NAME,
						value: token,
						reason: error.to_string(),
					},
					stack,
				))
			}
		}
	}
}