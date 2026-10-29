use std::{
	borrow::Cow,
	fmt::{
		Display,
		Write,
	},
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
	NoSuggestions,
	NodeInspector,
	NodeKind,
	NodeMetadata,
	RecoverableError,
	SuggestibleNode,
	SuggestionProvider,
	WriteNode,
};

pub struct Argument<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str = "", Suggester = NoSuggestions> {
	parser: Parser,
	children: Children,
	suggester: Suggester,
}

impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> WriteNode for Argument<NAME, Parser, Children, DESCRIPTION, Suggester>
where
	Parser: ArgParser,
{
	#[inline(always)]
	fn write_node<Buf>(buf: &mut Buf) -> core::fmt::Result
	where
		Buf: Write,
	{
		let name = std::any::type_name::<Parser::Output>();
		let value_type = if let Some((_, tail)) = name.rsplit_once("::") { tail } else { name };

		write!(buf, "<{NAME}: {value_type}>: {}", if DESCRIPTION.is_empty() { "No description provided." } else { DESCRIPTION })
	}
}

const impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str> Argument<NAME, Parser, Children, DESCRIPTION, NoSuggestions> {
	#[inline(always)]
	pub fn new(parser: Parser, children: Children) -> Self {
		Self {
			parser,
			children,
			suggester: NoSuggestions,
		}
	}
}

const impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> Argument<NAME, Parser, Children, DESCRIPTION, Suggester> {
	#[inline(always)]
	pub fn with_suggestions(parser: Parser, children: Children, suggester: Suggester) -> Self {
		Self { parser, children, suggester }
	}
}

const impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> Default for Argument<NAME, Parser, Children, DESCRIPTION, Suggester>
where
	Parser: [const] Default,
	Children: [const] Default,
	Suggester: [const] Default,
{
	#[inline(always)]
	fn default() -> Self {
		Self::with_suggestions(Parser::default(), Children::default(), Suggester::default())
	}
}

/// A node kind for an argument node.
pub struct ArgumentKind;

impl NodeKind for ArgumentKind {}

impl<const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> NodeMetadata for Argument<NAME, Parser, Children, DESCRIPTION, Suggester>
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

impl<Ctx, Stack, Source, const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> ExecutableNode<Ctx, Stack, Source>
	for Argument<NAME, Parser, Children, DESCRIPTION, Suggester>
where
	Source: InputSource,
	Parser: ArgParser,
	Parser::Error: Display,
	Children: ExecutableNode<Ctx, (Parser::Output, Stack), Source>,
{
	#[inline(always)]
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

impl<Source, const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> InspectableNode<Source> for Argument<NAME, Parser, Children, DESCRIPTION, Suggester>
where
	Source: InputSource,
	Parser: ArgParser,
	Parser::Error: Display,
	Children: NodeMetadata,
	<Children as NodeMetadata>::Inspect: InspectChild<Children, Source, Self>,
	<Children as NodeMetadata>::Kind: InspectMissingChild<Children, Source>,
{
	#[inline(always)]
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

#[cfg(not(feature = "async-suggestions"))]
impl<Ctx, const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> SuggestibleNode<Ctx> for Argument<NAME, Parser, Children, DESCRIPTION, Suggester>
where
	Parser: ArgParser,
	Children: SuggestibleNode<Ctx>,
	Suggester: SuggestionProvider<Ctx>,
{
	#[inline(always)]
	fn suggest<'c>(&'c self, context: &'c Ctx, completed: &[&str], prefix: &str, output: &mut Vec<Cow<'c, str>>) {
		if let Some((token, rest)) = completed.split_first() {
			if self.parser.parse(token).is_ok() {
				self.children.suggest(context, rest, prefix, output);
			}
		} else {
			output.extend(self.suggester.suggest(context, prefix));
		}
	}
}

#[cfg(feature = "async-suggestions")]
impl<Ctx, const NAME: &'static str, Parser, Children, const DESCRIPTION: &'static str, Suggester> SuggestibleNode<Ctx> for Argument<NAME, Parser, Children, DESCRIPTION, Suggester>
where
	Parser: ArgParser + Sync,
	Children: SuggestibleNode<Ctx>,
	Suggester: SuggestionProvider<Ctx>,
	Ctx: Sync,
{
	#[inline(always)]
	fn suggest<'c>(&'c self, context: &'c Ctx, completed: &[&str], prefix: &str, output: &mut Vec<Cow<'c, str>>) -> impl Future<Output = ()> + Send {
		async move {
			if let Some((token, rest)) = completed.split_first() {
				if self.parser.parse(token).is_ok() {
					self.children.suggest(context, rest, prefix, output).await;
				}
			} else {
				output.extend(self.suggester.suggest(context, prefix).await);
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
