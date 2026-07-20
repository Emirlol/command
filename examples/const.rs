use std::{
	assert_matches,
	convert::Infallible,
};

/// These imports are required due to the explicit type in cost field, unlike the basics.rs example
use command::{
	Argument,
	Choice,
	Exec,
	Literal,
	Root,
	WithExec,
};
use command::{
	CommandError,
	StringParser,
};
use macros::command;

// Same command tree from basics.rs, but const
const COMMAND: Root<
	Choice<
		Literal<"hello", WithExec<Argument<"name", StringParser, Exec<fn(&mut (), String) -> Result<(), Infallible>>>, Exec<fn(&mut ()) -> Result<(), Infallible>>>>,
		Choice<
			Literal<"hello2", Argument<"name", StringParser, Exec<fn(&mut (), String) -> Result<(), Infallible>>>>,
			Literal<
				"test",
				Argument<
					"arg1",
					StringParser,
					WithExec<
						Argument<"arg2", StringParser, Argument<"arg3", StringParser, Exec<fn(&mut (), String, String, String) -> Result<(), Infallible>>>>,
						Exec<fn(&mut (), String) -> Result<(), Infallible>>,
					>,
				>,
			>,
		>,
	>,
> = command! {
	literal "hello" {
		// Executors can be defined inline for leaf nodes, without requiring a new scope
		argument "name": StringParser executes |_: &mut (), name: String| {
			println!("Hello {name}!");
			Ok::<_, Infallible>(())
		}

		// Executors can also be defined as a separate entry in the node, this works for leaf nodes too but is the only way to add executors for non-leaf nodes
		executes |_: &mut ()| {
			println!("Hello world!");
			Ok::<_, Infallible>(())
		}
	}
	literal "hello2" {
		argument "name": StringParser executes test_print // Functions can be used as executors too, as long as they match the required signature
	}
	literal "test" {
		argument "arg1": StringParser {
			argument "arg2": StringParser {
				argument "arg3": StringParser executes |_: &mut (), arg1: String, arg2: String, arg3: String| {
					println!("arg1: {arg1}, arg2: {arg2}, arg3: {arg3}");
					Ok::<_, Infallible>(())
				}
			}

			executes |_: &mut (), arg1: String| {
				println!("arg1: {arg1}");
				Ok::<_, Infallible>(())
			}
		}
	}
};

fn main() {
	assert_matches!(COMMAND.execute(&mut (), "hello there"), Ok(()));
	assert_matches!(COMMAND.execute(&mut (), "hello"), Ok(()));
	assert_matches!(COMMAND.execute(&mut (), "hello2 function_test"), Ok(()));
	assert_matches!(COMMAND.execute(&mut (), "test hi there test"), Ok(()));
	assert_matches!(COMMAND.execute(&mut (), "test hi there"), Err(CommandError::IncompleteCommand)); // An executor was not defined for arg2 node, it will return an Err
	assert_matches!(COMMAND.execute(&mut (), "test hi"), Ok(()));
}

fn test_print(_: &mut (), name: String) -> Result<(), Infallible> {
	println!("Hello {name}!");
	Ok(())
}
