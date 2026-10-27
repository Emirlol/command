use crate::{
	Input,
	InputSource,
	error::DispatchError,
};

/// A node that can dispatch commands based on the input.
///
/// This is used for navigating the command tree.
pub(crate) trait DispatchNode<Ctx, Stack, Source>
where
	Source: InputSource,
{
	/// Dispatches the command based on the input and the current context.
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>>;
}
