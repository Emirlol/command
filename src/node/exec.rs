use command::{
	InspectionError,
	NodeInspector,
	NodeMetadata,
};

use crate::{
	CommandError,
	DispatchError,
	ExecutableNode,
	Executor,
	Input,
	InputSource,
	InspectableNode,
	IntoDispatchError,
	NoInspectKind,
	NoneInspect,
};

/// A terminal node that executes a function.
pub struct Exec<F> {
	executor: F,
}

impl<F> Exec<F> {
	#[inline(always)]
	pub const fn new(executor: F) -> Self {
		Self { executor }
	}
}

impl<F> Default for Exec<F>
where
	F: Default,
{
	fn default() -> Self {
		Self::new(F::default())
	}
}

#[cfg(feature = "const")]
impl<F> const_default::ConstDefault for Exec<F>
where
	F: const_default::ConstDefault,
{
	const DEFAULT: Self = Self::new(F::DEFAULT);
}

impl<F> NodeMetadata for Exec<F> {
	type Inspect = NoneInspect;
	type Kind = NoInspectKind;
}

impl<Ctx, Stack, Source, F> ExecutableNode<Ctx, Stack, Source> for Exec<F>
where
	Source: InputSource,
	F: Executor<Ctx, Stack>,
	F::Error: IntoDispatchError,
{
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		if let Some(tokens) = input.remaining_tokens() {
			return Err(DispatchError::Fatal(CommandError::TrailingInput(tokens.join(" "))));
		}

		self.executor.run(ctx, stack).map_err(IntoDispatchError::into_fatal) // IntoDispatchError has a specialization for CommandError to not box it again
	}
}

impl<Source: InputSource, F> InspectableNode<Source> for Exec<F> {
	fn inspect(&self, _inspector: &mut NodeInspector, _input: &mut Input<Source>) -> Result<(), InspectionError> {
		Err(InspectionError::IncompleteCommand)
	}
}
