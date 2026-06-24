use std::fmt::Display;

use crate::{
	args::ArgParser,
	CommandError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
	offset: usize,
}

#[derive(Debug)]
pub struct Input<'a> {
	source: &'a str,
	offset: usize,
}

impl<'a> Input<'a> {
	pub fn new(source: &'a str) -> Self {
		Self { source, offset: 0 }
	}

	pub fn checkpoint(&self) -> Checkpoint {
		Checkpoint { offset: self.offset }
	}

	pub fn restore(&mut self, checkpoint: Checkpoint) {
		self.offset = checkpoint.offset;
	}

	pub fn next_token(&mut self) -> Option<&'a str> {
		self.skip_whitespace();

		if self.offset >= self.source.len() {
			return None;
		}

		let start = self.offset;
		let rest = &self.source[start..];
		let end = rest.find(char::is_whitespace).map_or(self.source.len(), |idx| start + idx);
		self.offset = end;
		Some(&self.source[start..end])
	}

	pub fn is_empty(&self) -> bool {
		self.remaining_token().is_none()
	}

	pub fn remaining_token(&self) -> Option<&'a str> {
		let mut offset = self.offset;
		while let Some(ch) = self.source[offset..].chars().next() {
			if !ch.is_whitespace() {
				break;
			}
			offset += ch.len_utf8();
		}

		if offset >= self.source.len() {
			return None;
		}

		let rest = &self.source[offset..];
		let end = rest.find(char::is_whitespace).map_or(self.source.len(), |idx| offset + idx);
		Some(&self.source[offset..end])
	}

	fn skip_whitespace(&mut self) {
		while let Some(ch) = self.source[self.offset..].chars().next() {
			if !ch.is_whitespace() {
				break;
			}
			self.offset += ch.len_utf8();
		}
	}
}

pub trait CommandNode<Ctx, Stack> {
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), CommandError>;
}

pub trait Executor<Ctx, Stack> {
	fn run(&self, ctx: &mut Ctx, stack: Stack) -> Result<(), CommandError>;
}

pub struct Root<Children> {
	children: Children,
}

impl<Children> Root<Children> {
	pub fn new(children: Children) -> Self {
		Self { children }
	}

	pub fn execute<Ctx>(&self, ctx: &mut Ctx, input: &str) -> Result<(), CommandError>
	where
		Children: CommandNode<Ctx, ()>,
	{
		let mut input = Input::new(input);
		self.children.execute(ctx, &mut input, ())
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
	pub fn new(children: Children) -> Self {
		Self { children }
	}
}

impl<const NAME: &'static str, Children: Default> Default for Literal<NAME, Children> {
	fn default() -> Self {
		Self::new(Children::default())
	}
}

pub struct Argument<const NAME: &'static str, Parser, Children> {
	parser: Parser,
	children: Children,
}

impl<const NAME: &'static str, Parser, Children> Argument<NAME, Parser, Children> {
	pub fn new(parser: Parser, children: Children) -> Self {
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
	pub fn new(executor: F) -> Self {
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
	pub fn new(node: Node, executor: F) -> Self {
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
	pub fn new(head: Head, tail: Tail) -> Self {
		Self { head, tail }
	}
}

impl<Head: Default, Tail: Default> Default for Choice<Head, Tail> {
	fn default() -> Self {
		Self::new(Head::default(), Tail::default())
	}
}

enum DispatchError<Stack> {
	Recoverable(CommandError, Stack),
	Fatal(CommandError),
}

trait DispatchNode<Ctx, Stack> {
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), DispatchError<Stack>>;
}

impl<Ctx, Stack, Node> CommandNode<Ctx, Stack> for Node
where
	Node: DispatchNode<Ctx, Stack>,
{
	fn execute(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), CommandError> {
		match self.dispatch(ctx, input, stack) {
			Ok(()) => Ok(()),
			Err(DispatchError::Recoverable(error, _) | DispatchError::Fatal(error)) => Err(error),
		}
	}
}

impl<Ctx, Stack, const NAME: &'static str, Children> DispatchNode<Ctx, Stack> for Literal<NAME, Children>
where
	Children: DispatchNode<Ctx, Stack>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		match input.next_token() {
			Some(token) if token == NAME => self.children.dispatch(ctx, input, stack).map_err(DispatchError::into_fatal),
			Some(token) => {
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(CommandError::UnknownCommand(token.to_owned()), stack))
			}
			None => Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack)),
		}
	}
}

impl<Ctx, Stack, const NAME: &'static str, Parser, Children> DispatchNode<Ctx, Stack> for Argument<NAME, Parser, Children>
where
	Parser: ArgParser,
	Parser::Error: Display,
	Children: DispatchNode<Ctx, (Parser::Output, Stack)>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		let checkpoint = input.checkpoint();
		let Some(token) = input.next_token() else {
			return Err(DispatchError::Recoverable(CommandError::IncompleteCommand, stack));
		};

		match self.parser.parse(token) {
			Ok(value) => self.children.dispatch(ctx, input, (value, stack)).map_err(DispatchError::into_fatal_for_parent),
			Err(error) => {
				input.restore(checkpoint);
				Err(DispatchError::Recoverable(
					CommandError::InvalidArgument {
						name: NAME,
						value: token.to_owned(),
						reason: error.to_string(),
					},
					stack,
				))
			}
		}
	}
}

impl<Ctx, Stack, Node, F> DispatchNode<Ctx, Stack> for WithExec<Node, F>
where
	Node: DispatchNode<Ctx, Stack>,
	F: DispatchNode<Ctx, Stack>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		if input.is_empty() {
			self.executor.dispatch(ctx, input, stack)
		} else {
			self.node.dispatch(ctx, input, stack)
		}
	}
}

impl<Ctx, Stack, F> DispatchNode<Ctx, Stack> for Exec<F>
where
	F: Executor<Ctx, Stack>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), DispatchError<Stack>> {
		if let Some(token) = input.remaining_token() {
			return Err(DispatchError::Fatal(CommandError::TrailingInput(token.to_owned())));
		}

		self.executor.run(ctx, stack).map_err(DispatchError::Fatal)
	}
}

impl<Ctx, Stack> DispatchNode<Ctx, Stack> for Nil {
	fn dispatch(&self, _ctx: &mut Ctx, input: &mut Input<'_>, _stack: Stack) -> Result<(), DispatchError<Stack>> {
		if let Some(token) = input.remaining_token() {
			Err(DispatchError::Recoverable(CommandError::UnknownCommand(token.to_owned()), _stack))
		} else {
			Err(DispatchError::Recoverable(CommandError::IncompleteCommand, _stack))
		}
	}
}

impl<Ctx, Stack, Head, Tail> DispatchNode<Ctx, Stack> for Choice<Head, Tail>
where
	Head: DispatchNode<Ctx, Stack>,
	Tail: DispatchNode<Ctx, Stack>,
{
	fn dispatch(&self, ctx: &mut Ctx, input: &mut Input<'_>, stack: Stack) -> Result<(), DispatchError<Stack>> {
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
	fn into_fatal(self) -> DispatchError<Stack> {
		match self {
			Self::Recoverable(error, _) | Self::Fatal(error) => DispatchError::Fatal(error),
		}
	}

	fn into_fatal_for_parent<ParentStack>(self) -> DispatchError<ParentStack> {
		match self {
			Self::Recoverable(error, _) | Self::Fatal(error) => DispatchError::Fatal(error),
		}
	}
}

internal_macros::impl_executor!();

#[cfg(test)]
mod tests {
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
				Ok(())
			},
			|_ctx: &mut TestCtx, _player: String| Ok(()),
		);
		let mut ctx = TestCtx::default();

		command.execute(&mut ctx, "server stop").unwrap();

		assert_eq!(ctx.calls, ["stop"]);
	}

	#[test]
	fn unknown_literals_return_errors() {
		let command = server_command(|_ctx: &mut TestCtx| Ok(()), |_ctx: &mut TestCtx, _player: String| Ok(()));

		assert_eq!(command.execute(&mut TestCtx::default(), "proxy stop"), Err(CommandError::UnknownCommand("proxy".to_owned())),);
		assert_eq!(command.execute(&mut TestCtx::default(), "server restart"), Err(CommandError::UnknownCommand("restart".to_owned())),);
	}

	#[test]
	fn typed_arguments_flow_to_executor_in_source_order() {
		let command = server_command(
			|_ctx: &mut TestCtx| Ok(()),
			|ctx: &mut TestCtx, player: String| {
				ctx.calls.push(format!("ban:{player}"));
				Ok(())
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
			Ok(())
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
			Literal::<"stop", _>::new(Exec::new(|_ctx: &mut TestCtx| Ok(()))),
			Literal::<"start", _>::new(Exec::new(|ctx: &mut TestCtx| {
				ctx.calls.push("start".to_owned());
				Ok(())
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
				Ok(())
			})),
			Argument::<"value", _, _>::new(
				BoolParser,
				Exec::new(|ctx: &mut TestCtx, value: bool| {
					ctx.calls.push(format!("argument:{value}"));
					Ok(())
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
			Exec::new(|_ctx: &mut TestCtx, _value: i32| Ok(())),
		))));

		let err = command.execute(&mut TestCtx::default(), "server ban nope").unwrap_err();

		assert!(matches!(
			err,
			CommandError::InvalidArgument { name: "value", value, .. } if value == "nope"
		));
	}

	#[test]
	fn input_completion_is_enforced() {
		let command = server_command(|_ctx: &mut TestCtx| Ok(()), |_ctx: &mut TestCtx, _player: String| Ok(()));

		assert_eq!(command.execute(&mut TestCtx::default(), ""), Err(CommandError::IncompleteCommand),);
		assert_eq!(command.execute(&mut TestCtx::default(), "server"), Err(CommandError::IncompleteCommand),);
		assert_eq!(command.execute(&mut TestCtx::default(), "server stop now"), Err(CommandError::TrailingInput("now".to_owned())),);
	}

	#[test]
	fn bool_and_int_parser_errors_include_argument_name() {
		let bool_command = Root::new(Literal::<"server", _>::new(Argument::<"value", _, _>::new(
			BoolParser,
			Exec::new(|_ctx: &mut TestCtx, _value: bool| Ok(())),
		)));
		let int_command = Root::new(Literal::<"server", _>::new(Argument::<"value", _, _>::new(
			I32Parser,
			Exec::new(|_ctx: &mut TestCtx, _value: i32| Ok(())),
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
