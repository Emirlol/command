use crate::{
	DispatchError,
	ExecutableNode,
	Executor,
	Input,
	InputSource,
	InspectableNode,
	InspectionError,
	IntoDispatchError,
	NodeInspector,
	NodeMetadata,
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

const impl<Node, F> WithExec<Node, F> {
	#[inline(always)]
	pub fn new(node: Node, executor: F) -> Self {
		Self { node, executor }
	}
}

const impl<Node, F> Default for WithExec<Node, F>
where
	Node: [const] Default,
	F: [const] Default,
{
	#[inline(always)]
	fn default() -> Self {
		Self::new(Node::default(), F::default())
	}
}

impl<Ctx, Stack, Node, F, Source> ExecutableNode<Ctx, Stack, Source> for WithExec<Node, F>
where
	Source: InputSource,
	Node: ExecutableNode<Ctx, Stack, Source>,
	F: Executor<Ctx, Stack>,
	F::Error: IntoDispatchError,
{
	#[inline(always)]
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		if input.has_remaining() {
			self.node.execute(ctx, input, stack)
		} else {
			self.executor.run(ctx, stack).map_err(IntoDispatchError::into_fatal)
		}
	}
}

// Delegating impl to the child node since the executors aren't counted as actual nodes in the command tree.
impl<Node, F> NodeMetadata for WithExec<Node, F>
where
	Node: NodeMetadata,
{
	type Inspect = <Node as NodeMetadata>::Inspect;
	type Kind = <Node as NodeMetadata>::Kind;
}

impl<Node, F, Source> InspectableNode<Source> for WithExec<Node, F>
where
	Source: InputSource,
	Node: InspectableNode<Source>,
{
	#[inline(always)]
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
		self.node.inspect(inspector, input)
	}
}
