use std::{
	error::Error,
	fmt::Display,
};

use crate::{
	Input,
	InputSource,
	args::ArgParser,
	error::{
		CommandError,
		DispatchError,
		IntoDispatchError,
	},
	input::IntoInputSource,
};

pub trait CommandNode<Ctx, Stack, Source>
where
	Source: InputSource,
{
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), CommandError>;
}

pub trait Executor<Ctx, Stack> {
	type Error: Error + Send + Sync;
	fn run(&self, ctx: &mut Ctx, stack: Stack) -> Result<(), Self::Error>;
}

pub struct Root<Children> {
	children: Children,
}

impl<Children> Root<Children> {
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
		Children: CommandNode<Ctx, (), Source>,
	{
		let mut input = Input::new(input.into_input_source()?);
		self.children.execute(ctx, &mut input, ())
	}

	/// Executes the command tree with the given input source and `()` as the context.
	///
	/// See [IntoInputSource] for supported input types.
	#[inline(always)]
	pub fn execute_no_context<Source>(&self, input: impl IntoInputSource<Source = Source>) -> Result<(), CommandError>
	where
		Source: InputSource,
		Children: CommandNode<(), (), Source>,
	{
		self.execute(&mut (), input)
	}
}

impl<Children: Default> Default for Root<Children> {
	fn default() -> Self {
		Self::new(Children::default())
	}
}

pub struct Literal<const NAME: &'static str, Children> {
	children: Children,
}

impl<const NAME: &'static str, Children> Literal<NAME, Children> {
	pub const fn new(children: Children) -> Self {
		Self { children }
	}
}

impl<const NAME: &'static str, Children: Default> Default for Literal<NAME, Children> {
	fn default() -> Self {
		Self::new(Children::default())
	}
}

pub struct LiteralAliases<const NAMES: &'static [&'static str], Children> {
	children: Children,
}

impl<const NAMES: &'static [&'static str], Children> LiteralAliases<NAMES, Children> {
	pub const fn new(children: Children) -> Self {
		Self { children }
	}
}

impl<const NAMES: &'static [&'static str], Children: Default> Default for LiteralAliases<NAMES, Children> {
	fn default() -> Self {
		Self::new(Children::default())
	}
}

pub struct Argument<const NAME: &'static str, Parser, Children> {
	parser: Parser,
	children: Children,
}

impl<const NAME: &'static str, Parser, Children> Argument<NAME, Parser, Children> {
	pub const fn new(parser: Parser, children: Children) -> Self {
		Self { parser, children }
	}
}

impl<const NAME: &'static str, Parser: Default, Children: Default> Default for Argument<NAME, Parser, Children> {
	fn default() -> Self {
		Self::new(Parser::default(), Children::default())
	}
}

pub struct Exec<F> {
	executor: F,
}

impl<F> Exec<F> {
	pub const fn new(executor: F) -> Self {
		Self { executor }
	}
}

impl<F: Default> Default for Exec<F> {
	fn default() -> Self {
		Self::new(F::default())
	}
}

pub struct WithExec<Node, F> {
	node: Node,
	executor: F,
}

impl<Node, F> WithExec<Node, F> {
	pub const fn new(node: Node, executor: F) -> Self {
		Self { node, executor }
	}
}

#[derive(Default)]
pub struct Nil;

// Meant for Argument sibling nodes that linearly searches for a successful parse.
pub struct Choice<Head, Tail> {
	head: Head,
	tail: Tail,
}

impl<Head, Tail> Choice<Head, Tail> {
	pub const fn new(head: Head, tail: Tail) -> Self {
		Self { head, tail }
	}
}

impl<Head: Default, Tail: Default> Default for Choice<Head, Tail> {
	fn default() -> Self {
		Self::new(Head::default(), Tail::default())
	}
}

trait DispatchNode<Ctx, Stack, Source> {
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>>;
}

impl<Ctx, Stack, Node, Source> CommandNode<Ctx, Stack, Source> for Node
where
	Source: InputSource,
	Node: DispatchNode<Ctx, Stack, Source>,
{
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), CommandError> {
		match self.dispatch(ctx, input, stack) {
			Ok(()) => Ok(()),
			Err(DispatchError::Recoverable(error, _) | DispatchError::Fatal(error)) => Err(error),
		}
	}
}

impl<Ctx, Stack, const NAME: &'static str, Children, Source> DispatchNode<Ctx, Stack, Source> for Literal<NAME, Children>
where
	Source: InputSource,
	Children: DispatchNode<Ctx, Stack, Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		match input.next_token() {
			Some(token) if token == NAME => self.children.dispatch(ctx, input, stack).map_err(DispatchError::into_fatal),
			Some(token) => {
				let token = token.to_owned(); // Drop the mutable reference to input to allow restoring the checkpoint, otherwise it would be a double mutable borrow of input.
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(CommandError::UnknownCommand(token), stack))
			}
			None => Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack)),
		}
	}
}

impl<Ctx, Stack, const NAMES: &'static [&'static str], Children, Source> DispatchNode<Ctx, Stack, Source> for LiteralAliases<NAMES, Children>
where
	Source: InputSource,
	Children: DispatchNode<Ctx, Stack, Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();

		match input.next_token() {
			Some(token) if NAMES.iter().any(|name| token == *name) => self.children.dispatch(ctx, input, stack).map_err(DispatchError::into_fatal),
			Some(token) => {
				let token = token.to_owned(); // Drop the mutable reference to input to allow restoring the checkpoint, otherwise it would be a double mutable borrow of input.
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(CommandError::UnknownCommand(token), stack))
			}
			None => Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack)),
		}
	}
}

impl<Ctx, Stack, const NAME: &'static str, Parser, Children, Source> DispatchNode<Ctx, Stack, Source> for Argument<NAME, Parser, Children>
where
	Source: InputSource,
	Parser: ArgParser,
	Parser::Error: Display,
	Children: DispatchNode<Ctx, (Parser::Output, Stack), Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		let Some(token) = input.next_token() else {
			return Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack));
		};

		match self.parser.parse(token) {
			Ok(value) => self.children.dispatch(ctx, input, (value, stack)).map_err(DispatchError::into_fatal_for_parent),
			Err(error) => {
				// Drop the mutable reference to input to allow restoring the checkpoint, otherwise it would be a double mutable borrow of input.
				let token = token.to_owned();
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(
					CommandError::InvalidArgument {
						name: NAME,
						value: token,
						reason: error.to_string(),
					},
					stack,
				))
			}
		}
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

impl<Ctx, Stack, F, Source> DispatchNode<Ctx, Stack, Source> for Exec<F>
where
	Source: InputSource,
	F: Executor<Ctx, Stack>,
	F::Error: IntoDispatchError,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		if let Some(tokens) = input.remaining_tokens() {
			return Err(DispatchError::Fatal(CommandError::TrailingInput(tokens.join(" "))));
		}

		self.executor.run(ctx, stack).map_err(IntoDispatchError::into_fatal) // IntoDispatchError has a specialization for CommandError to not box it again
	}
}

impl<Ctx, Stack, Source> DispatchNode<Ctx, Stack, Source> for Nil
where
	Source: InputSource,
{
	fn dispatch(&self, _ctx: &mut Ctx, input: &mut Input<Source>, _stack: Stack) -> Result<(), DispatchError<Stack>> {
		if let Some(tokens) = input.remaining_tokens() {
			Err(DispatchError::Recoverable(CommandError::UnknownCommand(tokens.join(" ")), _stack))
		} else {
			Err(DispatchError::Recoverable(CommandError::IncompleteCommand, _stack))
		}
	}
}

impl<Ctx, Stack, Head, Tail, Source> DispatchNode<Ctx, Stack, Source> for Choice<Head, Tail>
where
	Source: InputSource,
	Head: DispatchNode<Ctx, Stack, Source>,
	Tail: DispatchNode<Ctx, Stack, Source>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<Source>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		match self.head.dispatch(ctx, input, stack) {
			Ok(()) => Ok(()),
			Err(DispatchError::Fatal(error)) => Err(DispatchError::Fatal(error)),
			Err(DispatchError::Recoverable(_, stack)) => {
				input.restore(checkpoint);
				self.tail.dispatch(ctx, input, stack)
			}
		}
	}
}

impl<Stack> DispatchError<Stack> {
	#[inline(always)]
	fn into_fatal(self) -> DispatchError<Stack> {
		match self {
			Self::Recoverable(error, _) | Self::Fatal(error) => DispatchError::Fatal(error),
		}
	}

	#[inline(always)]
	fn into_fatal_for_parent<ParentStack>(self) -> DispatchError<ParentStack> {
		match self {
			Self::Recoverable(error, _) | Self::Fatal(error) => DispatchError::Fatal(error),
		}
	}
}

internal_macros::impl_executor!();

#[cfg(test)]
mod tests {
	use std::assert_matches;

	use super::*;
	use crate::{
		BoolParser,
		I32Parser,
		StringParser,
	};

	#[derive(Default)]
	struct TestCtx {
		calls: Vec<String>,
	}

	type ServerCommand<Stop, Ban> = Root<Literal<"server", Choice<Literal<"stop", Exec<Stop>>, Choice<Literal<"ban", Argument<"player", StringParser, Exec<Ban>>>, Nil>>>>;

	fn server_command<Stop, Ban>(stop: Stop, ban: Ban) -> ServerCommand<Stop, Ban> {
		Root::new(Literal::new(Choice::new(
			Literal::<"stop", _>::new(Exec::new(stop)),
			Choice::new(Literal::<"ban", _>::new(Argument::<"player", _, _>::new(StringParser, Exec::new(ban))), Nil),
		)))
	}

	#[test]
	fn literal_dispatch_executes_matching_leaf() {
		let command = server_command(
			|ctx: &mut TestCtx| {
				ctx.calls.push("stop".to_owned());
				Ok::<(), CommandError>(())
			},
			|_ctx: &mut TestCtx, _player: String| Ok::<(), CommandError>(()),
		);
		let mut ctx = TestCtx::default();

		command.execute(&mut ctx, "server stop").unwrap();

		assert_eq!(ctx.calls, ["stop"]);
	}

	#[test]
	fn unknown_literals_return_errors() {
		let command = server_command(|_ctx: &mut TestCtx| Ok::<(), CommandError>(()), |_ctx: &mut TestCtx, _player: String| Ok::<(), CommandError>(()));

		match command.execute(&mut TestCtx::default(), "proxy stop") {
			Err(CommandError::UnknownCommand(token)) => assert_eq!(token, "proxy"),
			_ => panic!("Expected UnknownCommand error"),
		}
		match command.execute(&mut TestCtx::default(), "server restart") {
			Err(CommandError::UnknownCommand(token)) => assert_eq!(token, "restart"),
			_ => panic!("Expected UnknownCommand error"),
		}
	}

	#[test]
	fn typed_arguments_flow_to_executor_in_source_order() {
		let command = server_command(
			|_ctx: &mut TestCtx| Ok::<(), CommandError>(()),
			|ctx: &mut TestCtx, player: String| {
				ctx.calls.push(format!("ban:{player}"));
				Ok::<(), CommandError>(())
			},
		);
		let mut ctx = TestCtx::default();

		command.execute(&mut ctx, "server ban Steve").unwrap();

		assert_eq!(ctx.calls, ["ban:Steve"]);
	}

	#[test]
	fn executor_impl_supports_twenty_six_arguments() {
		let executor = |ctx: &mut TestCtx,
		                a: i32,
		                b: i32,
		                c: i32,
		                d: i32,
		                e: i32,
		                f: i32,
		                g: i32,
		                h: i32,
		                i: i32,
		                j: i32,
		                k: i32,
		                l: i32,
		                m: i32,
		                n: i32,
		                o: i32,
		                p: i32,
		                q: i32,
		                r: i32,
		                s: i32,
		                t: i32,
		                u: i32,
		                v: i32,
		                w: i32,
		                x: i32,
		                y: i32,
		                z: i32| {
			ctx.calls
				.push(format!("{a},{b},{c},{d},{e},{f},{g},{h},{i},{j},{k},{l},{m},{n},{o},{p},{q},{r},{s},{t},{u},{v},{w},{x},{y},{z}"));
			Ok::<(), CommandError>(())
		};
		let stack = (
			26,
			(
				25,
				(
					24,
					(
						23,
						(
							22,
							(21, (20, (19, (18, (17, (16, (15, (14, (13, (12, (11, (10, (9, (8, (7, (6, (5, (4, (3, (2, (1, ()))))))))))))))))))))),
						),
					),
				),
			),
		);
		let mut ctx = TestCtx::default();

		executor.run(&mut ctx, stack).unwrap();

		assert_eq!(ctx.calls, ["1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26"]);
	}

	#[test]
	fn sibling_branches_restore_cursor() {
		let command = Root::new(Literal::<"server", _>::new(Choice::new(
			Literal::<"stop", _>::new(Exec::new(|_ctx: &mut TestCtx| Ok::<(), CommandError>(()))),
			Literal::<"start", _>::new(Exec::new(|ctx: &mut TestCtx| {
				ctx.calls.push("start".to_owned());
				Ok::<(), CommandError>(())
			})),
		)));
		let mut ctx = TestCtx::default();

		command.execute(&mut ctx, "server start").unwrap();

		assert_eq!(ctx.calls, ["start"]);
	}

	#[test]
	fn literal_branch_precedes_argument_branch() {
		let command = Root::new(Literal::<"server", _>::new(Choice::new(
			Literal::<"true", _>::new(Exec::new(|ctx: &mut TestCtx| {
				ctx.calls.push("literal".to_owned());
				Ok::<(), CommandError>(())
			})),
			Argument::<"value", _, _>::new(
				BoolParser,
				Exec::new(|ctx: &mut TestCtx, value: bool| {
					ctx.calls.push(format!("argument:{value}"));
					Ok::<(), CommandError>(())
				}),
			),
		)));
		let mut ctx = TestCtx::default();

		command.execute(&mut ctx, "server true").unwrap();

		assert_eq!(ctx.calls, ["literal"]);
	}

	#[test]
	fn invalid_argument_after_matching_prefix_is_not_masked() {
		let command = Root::new(Literal::<"server", _>::new(Literal::<"ban", _>::new(Argument::<"value", _, _>::new(
			I32Parser,
			Exec::new(|_ctx: &mut TestCtx, _value: i32| Ok::<(), CommandError>(())),
		))));

		let err = command.execute(&mut TestCtx::default(), "server ban nope").unwrap_err();

		assert!(matches!(
			err,
			CommandError::InvalidArgument { name: "value", value, .. } if value == "nope"
		));
	}

	#[test]
	fn input_completion_is_enforced() {
		let command = server_command(|_ctx: &mut TestCtx| Ok::<(), CommandError>(()), |_ctx: &mut TestCtx, _player: String| Ok::<(), CommandError>(()));

		assert_matches!(command.execute(&mut TestCtx::default(), ""), Err(CommandError::IncompleteCommand));
		assert_matches!(command.execute(&mut TestCtx::default(), "server"), Err(CommandError::IncompleteCommand));
		match command.execute(&mut TestCtx::default(), "server stop now") {
			Err(CommandError::TrailingInput(token)) => assert_eq!(token, "now"),
			_ => panic!("Expected TrailingInput error"),
		}
	}

	#[test]
	fn bool_and_int_parser_errors_include_argument_name() {
		let bool_command = Root::new(Literal::<"server", _>::new(Argument::<"value", _, _>::new(
			BoolParser,
			Exec::new(|_ctx: &mut TestCtx, _value: bool| Ok::<(), CommandError>(())),
		)));
		let int_command = Root::new(Literal::<"server", _>::new(Argument::<"value", _, _>::new(
			I32Parser,
			Exec::new(|_ctx: &mut TestCtx, _value: i32| Ok::<(), CommandError>(())),
		)));

		assert!(matches!(
			bool_command.execute(&mut TestCtx::default(), "server maybe"),
			Err(CommandError::InvalidArgument { name: "value", value, .. }) if value == "maybe"
		));
		assert!(matches!(
			int_command.execute(&mut TestCtx::default(), "server 1.5"),
			Err(CommandError::InvalidArgument { name: "value", value, .. }) if value == "1.5"
		));
	}
}
