use std::fmt::Write;

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
	NodeInspector,
	NodeKind,
	NodeMetadata,
	RecoverableError,
	WriteNode,
};

/// A literal node, matched by a single string literal.
pub struct Literal<const NAME: &'static str, Children, const DESCRIPTION: &'static str = ""> {
	children: Children,
}

impl<const NAME: &'static str, Children, const DESCRIPTION: &'static str> WriteNode for Literal<NAME, Children, DESCRIPTION> {
	#[inline(always)]
	fn write_node<Buf>(buf: &mut Buf) -> std::fmt::Result
	where
		Buf: Write,
	{
		write!(buf, "{NAME}: {}", if DESCRIPTION.is_empty() { "No description provided." } else { DESCRIPTION })
	}
}

/// A literal node kind.
pub struct LiteralKind;

impl NodeKind for LiteralKind {}

impl<const NAME: &'static str, Children, const DESCRIPTION: &'static str> NodeMetadata for Literal<NAME, Children, DESCRIPTION> {
	type Inspect = HeadInspect;
	type Kind = LiteralKind;
}

impl<const NAME: &'static str, Children, const DESCRIPTION: &'static str> Literal<NAME, Children, DESCRIPTION> {
	#[inline(always)]
	pub const fn new(children: Children) -> Self {
		Self { children }
	}
}

const impl<const NAME: &'static str, Children, const DESCRIPTION: &'static str> Default for Literal<NAME, Children, DESCRIPTION>
where
	Children: [const] Default,
{
	#[inline(always)]
	fn default() -> Self {
		Self::new(Children::default())
	}
}

#[cfg(feature = "const")]
impl<const NAME: &'static str, Children, const DESCRIPTION: &'static str> const_default::ConstDefault for Literal<NAME, Children, DESCRIPTION>
where
	Children: const_default::ConstDefault,
{
	const DEFAULT: Self = Self::new(Children::DEFAULT);
}

impl<Child, Source> InspectMissingChild<Child, Source> for LiteralKind
where
	Source: InputSource,
{
	#[inline(always)]
	fn inspect_missing(_child: &Child, _inspector: &mut NodeInspector, _input: &mut Input<Source>) -> Result<(), InspectionError> {
		Ok(())
	}
}

impl<Ctx, Stack, Source, const NAME: &'static str, Children, const DESCRIPTION: &'static str> ExecutableNode<Ctx, Stack, Source> for Literal<NAME, Children, DESCRIPTION>
where
	Source: InputSource,
	Children: ExecutableNode<Ctx, Stack, Source>,
{
	#[inline(always)]
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		match input.peek() {
			Some(token) if token == NAME => {
				input.advance(1);
				self.children.execute(ctx, input, stack).map_err(|e| match e {
					DispatchError::Recoverable(error, _) => DispatchError::Fatal(error.into_command_error(input)),
					DispatchError::Fatal(error) => DispatchError::Fatal(error),
				})
			}
			Some(_) => Err(DispatchError::Recoverable(RecoverableError::UnknownCommand { offset: input.offset }, stack)),
			None => Err(DispatchError::Recoverable(RecoverableError::IncompleteCommand, stack)),
		}
	}
}

impl<Source, const NAME: &'static str, Children, const DESCRIPTION: &'static str> InspectableNode<Source> for Literal<NAME, Children, DESCRIPTION>
where
	Source: InputSource,
	Children: NodeMetadata,
	<Children as NodeMetadata>::Inspect: InspectChild<Children, Source, Self>,
	<Children as NodeMetadata>::Kind: InspectMissingChild<Children, Source>,
{
	#[inline(always)]
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		match input.peek() {
			Some(token) if token == NAME => {
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
		Nil,
		NodeInspector,
		RecoverableError,
		U32Parser,
		WriteNode,
	};

	#[test]
	fn literals_dispatch_correctly() {
		fn write(output: &mut String) -> Result<(), CommandError> {
			output.push_str("stopped");
			Ok(())
		}

		type Node = Literal<"stop", Exec<fn(&mut String) -> Result<(), CommandError>>, "Stops the current process">;
		let node = Node::new(Exec::new(write));
		let mut output = String::new();

		assert_matches!(node.execute(&mut output, &mut Input::new("stop".into_input_source().unwrap()), ()), Ok(()));
		assert_eq!(output, "stopped");
		output.clear();

		assert_matches!(node.execute(&mut output, &mut Input::new("invalid".into_input_source().unwrap()), ()), Err(DispatchError::Recoverable(RecoverableError::UnknownCommand { offset }, _)) if offset == 0);
		assert_eq!(output, "");
	}

	#[test]
	fn literals_inspect_correctly() {
		type Node = Literal<
			"stop",
			Choice<Literal<"now", Nil, "Stops immediately">, Literal<"later", Argument<"delay", U32Parser, Nil, "The delay in seconds">, "Stops after a delay">>,
			"Stops the current process",
		>;

		let node = Node::default();

		let mut inspector = NodeInspector::new();
		assert_matches!(node.inspect(&mut inspector, &mut Input::new("stop".into_input_source().unwrap())), Ok(()));
		assert_eq!(
			inspector.output,
			"  stop: Stops the current process\n    now: Stops immediately\n    later: Stops after a delay\n      <delay: u32>: The delay in seconds\n"
		);

		inspector.reset();
		assert_matches!(node.inspect(&mut inspector, &mut Input::new("stop later".into_input_source().unwrap())), Ok(()));
		assert_eq!(
			inspector.output,
			"  stop: Stops the current process\n    later: Stops after a delay\n      <delay: u32>: The delay in seconds\n"
		);

		inspector.reset();
		assert_matches!(node.inspect(&mut inspector, &mut Input::new("invalid".into_input_source().unwrap())), Err(InspectionError::UnknownCommand { offset }) if offset == 0);
		assert_eq!(inspector.output, "");
	}

	#[test]
	fn aliases_write_as_one_line_for_help() {
		type Node = Literal<"stop", Nil, "Stops the current process">;
		let mut output = String::new();

		Node::write_node(&mut output).unwrap();

		assert_eq!(output, "stop: Stops the current process");
	}
}
