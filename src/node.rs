mod root;
pub use root::*;
mod literal;
pub use literal::*;
mod literal_aliases;
pub use literal_aliases::*;
mod argument;
pub use argument::*;
mod exec;
pub use exec::*;
mod with_exec;
pub use with_exec::*;
mod nil;
pub use nil::*;
mod choice;
pub use choice::*;
mod command_node;
pub use command_node::*;
mod dispatch_node;

use crate::error::DispatchError;

pub trait Executor<Ctx, Stack> {
	type Error: std::error::Error + Send + Sync;
	fn run(&self, ctx: &mut Ctx, stack: Stack) -> Result<(), Self::Error>;
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

internal_macros::impl_executor!(26); // Support stacks up to 26 deep -- this means there can be up to 26 consecutive argument nodes in a command. If this is not enough, you're doing something weird.

#[cfg(test)]
mod tests {
	use std::assert_matches;

	use super::*;
	use crate::*;

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
