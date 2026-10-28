use crate::{
	DispatchNode,
	Input,
	InputSource,
	error::DispatchError,
};

/// A non-terminal node that can have 2 children:
/// - The first child is the main "child" node that can be matched against
/// - The terminal executor in case the first child node failed to match.
///
/// `WithExec` can be thought of as `Choice<Child, Exec<F>>`, but it describes the intent better while having a simpler implementation.
///
/// This allows creating intermediary nodes that can execute a function if their child node fails to match.
/// The main use case is for optional subcommands, where the parent node can be executed as is.
///
/// An example implementation might be:
/// `help` - executes the help command which lists all possible commands
/// `help <command>` - executes the help command for a specific command, which lists all possible subcommands and arguments for that command
pub struct WithExec<Node, F> {
	node: Node,
	executor: F,
}

impl<Node, F> WithExec<Node, F> {
	pub const fn new(node: Node, executor: F) -> Self {
		Self { node, executor }
	}
}

impl<Ctx, Stack, Node, F, Source> DispatchNode<Ctx, Stack, Source> for WithExec<Node, F>
where
	Source: InputSource,
	Node: DispatchNode<Ctx, Stack, Source>,
	F: DispatchNode<Ctx, Stack, Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		if input.has_remaining() {
			self.node.dispatch(ctx, input, stack)
		} else {
			self.executor.dispatch(ctx, input, stack)
		}
	}
}
