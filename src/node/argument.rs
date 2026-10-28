use std::fmt::Display;

use crate::{
	ArgParser,
	DispatchNode,
	Input,
	InputSource,
	error::{
		DispatchError,
		RecoverableError,
	},
};

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
		let Some(token) = input.peek() else {
			return Err(DispatchError::Recoverable(RecoverableError::IncompleteCommand, stack));
		};

		match self.parser.parse(token) {
			Ok(value) => {
				input.advance(1);
				self.children.dispatch(ctx, input, (value, stack)).map_err(|e| e.into_fatal_for_parent(input))
			}
			Err(error) => Err(DispatchError::Recoverable(
				RecoverableError::InvalidArgument {
					name: NAME,
					offset: input.offset,
					error: error.to_string(),
				},
				stack,
			)),
		}
	}
}
