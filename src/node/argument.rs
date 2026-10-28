use std::fmt::{
	Display,
	Write,
};

use crate::{
	ArgParser,
	DispatchError,
	ExecutableNode,
	HeadInspect,
	Input,
	InputSource,
	InspectChild,
	InspectMissingChild,
	InspectableNode,
	InspectionError,
	NodeInspector,
	NodeKind,
	NodeMetadata,
	RecoverableError,
	WriteNode,
};

pub struct Argument<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str = ""> {
	parser: Parser,
	children: Children,
}

impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> WriteNode for Argument<NAME, Parser, Children, DESCRIPTION>
where
	Parser: ArgParser,
{
	fn write_node<Buf>(buf: &mut Buf) -> core::fmt::Result
	where
		Buf: Write,
	{
		let name = std::any::type_name::<Parser::Output>();
		let value_type = if let Some((_, tail)) = name.rsplit_once("::") { tail } else { name };

		write!(buf, "<{NAME}: {value_type}>: {}", if DESCRIPTION.is_empty() { "No description provided." } else { DESCRIPTION })
	}
}

impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> Argument<NAME, Parser, Children, DESCRIPTION> {
	#[inline(always)]
	pub const fn new(parser: Parser, children: Children) -> Self {
		Self { parser, children }
	}
}

impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> Default for Argument<NAME, Parser, Children, DESCRIPTION>
where
	Parser: Default,
	Children: Default,
{
	fn default() -> Self {
		Self::new(Parser::default(), Children::default())
	}
}

#[cfg(feature = "const")]
impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> const_default::ConstDefault for Argument<NAME, Parser, Children, DESCRIPTION>
where
	Parser: const_default::ConstDefault,
	Children: const_default::ConstDefault,
{
	const DEFAULT: Self = Self::new(Parser::DEFAULT, Children::DEFAULT);
}

/// A node kind for an argument node.
pub struct ArgumentKind;

impl NodeKind for ArgumentKind {}

impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> NodeMetadata for Argument<NAME, Parser, Children, DESCRIPTION>
where
	Parser: ArgParser,
{
	type Inspect = HeadInspect;
	type Kind = ArgumentKind;
}

impl<Children, Source> InspectMissingChild<Children, Source> for ArgumentKind
where
	Source: InputSource,
	Children: InspectableNode<Source>,
{
	#[inline(always)]
	fn inspect_missing(child: &Children, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		inspector.at_child(|inspector| child.inspect(inspector, input))
	}
}

impl<Ctx, Stack, Source, const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> ExecutableNode<Ctx, Stack, Source> for Argument<NAME, Parser, Children, DESCRIPTION>
where
	Source: InputSource,
	Parser: ArgParser,
	Parser::Error: Display,
	Children: ExecutableNode<Ctx, (Parser::Output, Stack), Source>,
{
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let Some(token) = input.peek() else {
			return Err(DispatchError::Recoverable(RecoverableError::IncompleteCommand, stack));
		};

		match self.parser.parse(token) {
			Ok(value) => {
				input.advance(1);
				self.children.execute(ctx, input, (value, stack)).map_err(|e| e.into_fatal_for_parent(input))
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

impl<Source, const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> InspectableNode<Source> for Argument<NAME, Parser, Children, DESCRIPTION>
where
	Source: InputSource,
	Parser: ArgParser,
	Parser::Error: Display,
	Children: NodeMetadata,
	<Children as NodeMetadata>::Inspect: InspectChild<Children, Source, Self>,
	<Children as NodeMetadata>::Kind: InspectMissingChild<Children, Source>,
{
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		let Some(token) = input.peek() else {
			inspector.inspect::<Self>()?;
			return <<Children as NodeMetadata>::Kind as InspectMissingChild<Children, Source>>::inspect_missing(&self.children, inspector, input);
		};

		match self.parser.parse(token) {
			Ok(_) => {
				input.advance(1);
				inspector.inspect::<Self>()?;
				<<Children as NodeMetadata>::Inspect as InspectChild<Children, Source, Self>>::inspect_child(&self.children, inspector, input)
			}
			Err(_) => {
				// There was some input in this position, but it was invalid. We still want to inspect the argument node to show the user what was expected here.
				inspector.inspect::<Self>()
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{
		assert_matches,
		convert::Infallible,
	};

	use crate::{
		Argument,
		Exec,
		ExecutableNode,
		Input,
		IntoInputSource,
		StringParser,
	};

	#[test]
	fn execute_argument_node() {
		type Node = Argument<"arg", StringParser, Exec<fn(&mut Option<String>, String) -> Result<(), Infallible>>, "A string argument">;

		let node = Node::new(
			StringParser,
			Exec::new(|ctx: &mut Option<String>, arg: String| {
				*ctx = Some(arg);
				Ok(())
			}),
		);

		let mut ctx = Option::<String>::None;
		assert_matches!(node.execute(&mut ctx, &mut Input::new("test".into_input_source().unwrap()), ()), Ok(()));
		assert_eq!(ctx, Some("test".to_string()));
	}
}
