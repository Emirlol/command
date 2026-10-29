use std::{
	borrow::Cow,
	fmt::Write,
};

use crate::{
	DispatchError,
	ExecutableNode,
	HeadInspect,
	Input,
	InputSource,
	InspectChild,
	InspectMissingChild,
	InspectableNode,
	InspectionError,
	LiteralKind,
	NodeInspector,
	NodeMetadata,
	RecoverableError,
	SuggestibleNode,
	WriteNode,
};

/// A literal node that can have match multiple string literals.
///
/// The main use case is to not replicate children nodes in the command tree by creating a graph that consists of 1-n-1 layers instead of 1-n-n.
pub struct LiteralAliases<const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str = ""> {
	children: Children,
}

const impl<const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> LiteralAliases<NAMES, Children, DESCRIPTION> {
	#[inline(always)]
	pub fn new(children: Children) -> Self {
		Self { children }
	}
}

const impl<const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> Default for LiteralAliases<NAMES, Children, DESCRIPTION>
where
	Children: [const] Default,
{
	#[inline(always)]
	fn default() -> Self {
		Self::new(Children::default())
	}
}

impl<const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> WriteNode for LiteralAliases<NAMES, Children, DESCRIPTION> {
	#[inline(always)]
	fn write_node<Buf>(buf: &mut Buf) -> std::fmt::Result
	where
		Buf: Write,
	{
		let names = NAMES.join(" | ");
		write!(buf, "{names}: {}", if DESCRIPTION.is_empty() { "No description provided." } else { DESCRIPTION })
	}
}

impl<Ctx, Stack, Source, const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> ExecutableNode<Ctx, Stack, Source> for LiteralAliases<NAMES, Children, DESCRIPTION>
where
	Source: InputSource,
	Children: ExecutableNode<Ctx, Stack, Source>,
{
	#[inline(always)]
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		match input.peek() {
			Some(ref token) if NAMES.contains(token) => {
				input.advance(1);
				self.children.execute(ctx, input, stack).map_err(|e| e.into_fatal(input))
			}
			Some(_) => Err(DispatchError::Recoverable(RecoverableError::UnknownCommand { offset: input.offset }, stack)),
			None => Err(DispatchError::Recoverable(RecoverableError::IncompleteCommand, stack)),
		}
	}
}

impl<const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> NodeMetadata for LiteralAliases<NAMES, Children, DESCRIPTION> {
	type Inspect = HeadInspect;
	type Kind = LiteralKind;
}

impl<Source, const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> InspectableNode<Source> for LiteralAliases<NAMES, Children, DESCRIPTION>
where
	Source: InputSource,
	Children: NodeMetadata,
	<Children as NodeMetadata>::Inspect: InspectChild<Children, Source, Self>,
	<Children as NodeMetadata>::Kind: InspectMissingChild<Children, Source>,
{
	#[inline(always)]
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		match input.peek() {
			Some(ref token) if NAMES.contains(token) => {
				input.advance(1);
				inspector.inspect::<Self>()?;

				<<Children as NodeMetadata>::Inspect as InspectChild<Children, Source, Self>>::inspect_child(&self.children, inspector, input)
			}
			Some(_) => Err(InspectionError::UnknownCommand { offset: input.offset }),
			None => {
				inspector.inspect::<Self>()?;
				<<Children as NodeMetadata>::Kind as InspectMissingChild<Children, Source>>::inspect_missing(&self.children, inspector, input)
			}
		}
	}
}

#[cfg(not(feature = "async-suggestions"))]
impl<Ctx, const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> SuggestibleNode<Ctx> for LiteralAliases<NAMES, Children, DESCRIPTION>
where
	Children: SuggestibleNode<Ctx>,
{
	#[inline(always)]
	fn suggest<'c>(&'c self, context: &'c Ctx, completed: &[&str], prefix: &str, output: &mut Vec<Cow<'c, str>>) {
		if let Some((token, rest)) = completed.split_first() {
			if NAMES.contains(token) {
				self.children.suggest(context, rest, prefix, output);
			}
		} else {
			output.extend(NAMES.iter().filter(|name| name.starts_with(prefix)).map(|name| Cow::Borrowed(*name)));
		}
	}
}

#[cfg(feature = "async-suggestions")]
impl<Ctx, const NAMES: &'static [&'static str], Children, const DESCRIPTION: &'static str> SuggestibleNode<Ctx> for LiteralAliases<NAMES, Children, DESCRIPTION>
where
	Children: SuggestibleNode<Ctx>,
	Ctx: Sync,
{
	#[inline(always)]
	fn suggest<'c>(&'c self, context: &'c Ctx, completed: &[&str], prefix: &str, output: &mut Vec<Cow<'c, str>>) -> impl Future<Output = ()> + Send {
		async move {
			if let Some((token, rest)) = completed.split_first() {
				if NAMES.contains(token) {
					self.children.suggest(context, rest, prefix, output).await;
				}
			} else {
				output.extend(NAMES.iter().filter(|name| name.starts_with(prefix)).map(|name| Cow::Borrowed(*name)));
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::assert_matches;

	use crate::{
		Argument,
		Choice,
		CommandError,
		DispatchError,
		Exec,
		ExecutableNode,
		Input,
		InspectableNode,
		InspectionError,
		IntoInputSource,
		Literal,
		LiteralAliases,
		Nil,
		NodeInspector,
		RecoverableError,
		U32Parser,
		WriteNode,
	};

	#[test]
	fn aliases_dispatch_correctly() {
		fn write(output: &mut Option<&'static str>) -> Result<(), CommandError> {
			*output = Some("stopped");
			Ok(())
		}

		type Node = LiteralAliases<{ &["stop", "quit", "q"] }, Exec<fn(&mut Option<&'static str>) -> Result<(), CommandError>>, "Stops the current process">;
		let node = Node::new(Exec::new(write));
		let mut output = Option::<&'static str>::None;

		assert_matches!(node.execute(&mut output, &mut Input::new("stop".into_input_source().unwrap()), ()), Ok(()));
		assert_matches!(output, Some("stopped"));
		output = None;

		assert_matches!(node.execute(&mut output, &mut Input::new("quit".into_input_source().unwrap()), ()), Ok(()));
		assert_matches!(output, Some("stopped"));
		output = None;

		assert_matches!(node.execute(&mut output, &mut Input::new("q".into_input_source().unwrap()), ()), Ok(()));
		assert_matches!(output, Some("stopped"));
		output = None;

		assert_matches!(node.execute(&mut output, &mut Input::new("invalid".into_input_source().unwrap()), ()), Err(DispatchError::Recoverable(RecoverableError::UnknownCommand { offset }, _)) if offset == 0);
		assert_matches!(output, None);
	}

	#[test]
	fn aliases_inspect_correctly() {
		type Node = LiteralAliases<
			{ &["stop", "quit", "q"] },
			Choice<LiteralAliases<{ &["now", "immediately"] }, Nil, "Stops immediately">, Literal<"later", Argument<"delay", U32Parser, Nil, "The delay in seconds">, "Stops after a delay">>,
			"Stops the current process",
		>;

		let node = Node::default();

		let mut inspector = NodeInspector::new();
		assert_matches!(node.inspect(&mut inspector, &mut Input::new("stop".into_input_source().unwrap())), Ok(()));
		assert_eq!(
			inspector.output,
			"  stop | quit | q: Stops the current process\n    now | immediately: Stops immediately\n    later: Stops after a delay\n      <delay: u32>: The delay in seconds\n"
		);

		inspector.reset();
		assert_matches!(node.inspect(&mut inspector, &mut Input::new("quit".into_input_source().unwrap())), Ok(()));
		assert_eq!(
			inspector.output,
			"  stop | quit | q: Stops the current process\n    now | immediately: Stops immediately\n    later: Stops after a delay\n      <delay: u32>: The delay in seconds\n"
		);

		inspector.reset();
		assert_matches!(node.inspect(&mut inspector, &mut Input::new("q".into_input_source().unwrap())), Ok(()));
		assert_eq!(
			inspector.output,
			"  stop | quit | q: Stops the current process\n    now | immediately: Stops immediately\n    later: Stops after a delay\n      <delay: u32>: The delay in seconds\n"
		);

		inspector.reset();
		assert_matches!(node.inspect(&mut inspector, &mut Input::new("invalid".into_input_source().unwrap())), Err(InspectionError::UnknownCommand { offset }) if offset == 0);
		assert_eq!(inspector.output, "");
	}

	#[test]
	fn aliases_write_as_one_line_for_help() {
		type Node = LiteralAliases<{ &["stop", "quit", "q"] }, Nil, "Stops the current process">;
		let mut output = String::new();

		Node::write_node(&mut output).unwrap();

		assert_eq!(output, "stop | quit | q: Stops the current process");
	}
}
