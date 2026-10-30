use crate::{
	BothInspect,
	CombineInspect,
	DispatchError,
	ExecutableNode,
	HeadInspect,
	Input,
	InputSource,
	InspectableNode,
	InspectionError,
	NodeInspector,
	NodeMetadata,
	NoneInspect,
	TailInspect,
};

/// A node that has 2 child nodes.
///
/// This is the main building block of a command tree, allowing for branching between different command paths.
pub struct Choice<Head, Tail> {
	head: Head,
	tail: Tail,
}

const impl<Head, Tail> Choice<Head, Tail>
where
	Head: NodeMetadata,
	Tail: NodeMetadata,
{
	#[inline(always)]
	pub fn new(head: Head, tail: Tail) -> Self {
		Self { head, tail }
	}
}

const impl<Head, Tail> Default for Choice<Head, Tail>
where
	Head: [const] Default + NodeMetadata,
	Tail: [const] Default + NodeMetadata,
{
	#[inline(always)]
	fn default() -> Self {
		Self::new(Head::default(), Tail::default())
	}
}

impl<Head, Tail> NodeMetadata for Choice<Head, Tail>
where
	Head: NodeMetadata,
	Tail: NodeMetadata,
	<Head as NodeMetadata>::Inspect: CombineInspect<<Tail as NodeMetadata>::Inspect>,
{
	type Inspect = <<Head as NodeMetadata>::Inspect as CombineInspect<<Tail as NodeMetadata>::Inspect>>::Output;
	type Kind = <Head as NodeMetadata>::Kind;
}

impl<Ctx, Stack, Source, Head, Tail> ExecutableNode<Ctx, Stack, Source> for Choice<Head, Tail>
where
	Source: InputSource,
	Head: ExecutableNode<Ctx, Stack, Source>,
	Tail: ExecutableNode<Ctx, Stack, Source>,
{
	#[inline(always)]
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		match self.head.execute(ctx, input, stack) {
			Ok(()) => Ok(()),
			Err(DispatchError::Recoverable(_, stack)) => self.tail.execute(ctx, input, stack),
			Err(DispatchError::Fatal(error)) => Err(DispatchError::Fatal(error)),
		}
	}
}

impl<Source, Head, Tail> InspectableNode<Source, NoneInspect> for Choice<Head, Tail>
where
	Source: InputSource,
	Head: NodeMetadata,
	Tail: NodeMetadata,
	<Head as NodeMetadata>::Inspect: CombineInspect<<Tail as NodeMetadata>::Inspect>,
{
	#[inline(always)]
	fn inspect(&self, _inspector: &mut NodeInspector, _input: &mut Input<Source>) -> Result<(), InspectionError> {
		Ok(())
	}
}

impl<Source, Head, Tail> InspectableNode<Source, HeadInspect> for Choice<Head, Tail>
where
	Source: InputSource,
	Head: InspectableNode<Source> + NodeMetadata,
	Tail: NodeMetadata,
	<Head as NodeMetadata>::Inspect: CombineInspect<<Tail as NodeMetadata>::Inspect>,
{
	#[inline(always)]
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		self.head.inspect(inspector, input)
	}
}

impl<Source, Head, Tail> InspectableNode<Source, TailInspect> for Choice<Head, Tail>
where
	Source: InputSource,
	Tail: InspectableNode<Source> + NodeMetadata,
	Head: NodeMetadata,
	<Head as NodeMetadata>::Inspect: CombineInspect<<Tail as NodeMetadata>::Inspect>,
{
	#[inline(always)]
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		self.tail.inspect(inspector, input)
	}
}

impl<Source, Head, Tail> InspectableNode<Source, BothInspect> for Choice<Head, Tail>
where
	Source: InputSource,
	Head: InspectableNode<Source> + NodeMetadata,
	Tail: InspectableNode<Source> + NodeMetadata,
	<Head as NodeMetadata>::Inspect: CombineInspect<<Tail as NodeMetadata>::Inspect>,
{
	#[inline(always)]
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		let start = input.checkpoint();
		let mut head_inspector = inspector.branch();
		let head_result = self.head.inspect(&mut head_inspector, input);
		let head_offset = input.offset;

		input.restore(start);
		let mut tail_inspector = inspector.branch();
		let tail_result = self.tail.inspect(&mut tail_inspector, input);
		let tail_offset = input.offset;

		match (head_result, tail_result) {
			(Ok(()), Ok(())) => {
				inspector.append(head_inspector);
				inspector.append(tail_inspector);
				input.restore(if head_offset >= tail_offset {
					crate::Checkpoint { offset: head_offset }
				} else {
					crate::Checkpoint { offset: tail_offset }
				});
				Ok(())
			}
			(Ok(()), Err(_)) => {
				inspector.append(head_inspector);
				input.restore(crate::Checkpoint { offset: head_offset });
				Ok(())
			}
			(Err(_), Ok(())) => {
				inspector.append(tail_inspector);
				input.restore(crate::Checkpoint { offset: tail_offset });
				Ok(())
			}
			(Err(head_error), Err(tail_error)) => {
				input.restore(start);
				match (head_error, tail_error) {
					(InspectionError::UnknownCommand { offset: a }, InspectionError::UnknownCommand { offset: b }) => Err(InspectionError::UnknownCommand { offset: a.max(b) }),
					(InspectionError::UnknownCommand { offset }, _) | (_, InspectionError::UnknownCommand { offset }) => Err(InspectionError::UnknownCommand { offset }),
					(InspectionError::Input(error), _) | (_, InspectionError::Input(error)) => Err(InspectionError::Input(error)),
					_ => Err(InspectionError::IncompleteCommand),
				}
			}
		}
	}
}
