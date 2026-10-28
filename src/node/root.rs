use crate::{
	CommandError,
	DispatchNode,
	Input,
	InputSource,
	IntoInputSource,
	error::DispatchError,
};

/// The root of a command tree.
pub struct Root<Children> {
	children: Children,
}

impl<Children> Root<Children> {
	#[inline(always)]
	pub const fn new(children: Children) -> Self {
		Self { children }
	}

	/// Executes the command tree with the given context and input source.
	///
	/// If there's no context to pass to the executor, you can alternatively use [`execute_no_context`] which will use `()` as the context type.
	///
	/// See [IntoInputSource] for supported input types.
	pub fn execute<Ctx, Source>(&self, ctx: &mut Ctx, input: impl IntoInputSource<Source = Source>) -> Result<(), CommandError>
	where
		Source: InputSource,
		Children: DispatchNode<Ctx, (), Source>,
	{
		let mut input = Input::new(input.into_input_source()?);
		match self.children.dispatch(ctx, &mut input, ()) {
			Ok(()) => Ok(()),
			Err(DispatchError::Recoverable(error, _)) => Err(error.into_command_error(&input)),
			Err(DispatchError::Fatal(error)) => Err(error),
		}
	}

	/// Executes the command tree with the given input source and `()` as the context.
	///
	/// See [IntoInputSource] for supported input types.
	#[inline(always)]
	pub fn execute_no_context<Source>(&self, input: impl IntoInputSource<Source = Source>) -> Result<(), CommandError>
	where
		Source: InputSource,
		Children: DispatchNode<(), (), Source>,
	{
		self.execute(&mut (), input)
	}
}

impl<Children: Default> Default for Root<Children> {
	fn default() -> Self {
		Self::new(Children::default())
	}
}

#[cfg(feature = "const")]
impl<Children: const_default::ConstDefault> const_default::ConstDefault for Root<Children> {
	const DEFAULT: Self = Self::new(Children::DEFAULT);
}
