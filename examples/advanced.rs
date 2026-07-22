use std::{
	assert_matches,
	sync::mpsc::Sender,
};

// These imports are normally not required, but since the example is within the same project as the main lib, the macro impl generates crate:: prefixed paths which break in this example
use command::{
	ArgParser,
	BoolParser,
	CommandError,
	ParseError,
	StringParser,
	command_error,
};
// These imports are normally not required, but since the example is within the same project as the main lib, the macro impl generates crate:: prefixed paths which break in this example
use command::{
	Argument,
	Choice,
	Exec,
	Literal,
	Root,
	WithExec,
};
use macros::command;

#[derive(Debug)]
struct Ban {
	name: String,
	reason: BanReason,
}

#[derive(Default, Debug)]
enum BanReason {
	#[default]
	Nothing,
	AbsolutelyNothing,
}

// Parsers can be defined for other types besides the primitive types, this is useful for complex types you want to be able to parse from the command line.
#[derive(Default)]
struct BanReasonParser;

impl ArgParser for BanReasonParser {
	type Output = BanReason;
	type Error = ParseError;

	fn parse(&self, input: &str) -> Result<Self::Output, Self::Error> {
		match input {
			"nothing" => Ok(BanReason::Nothing),
			"absolutely_nothing" => Ok(BanReason::AbsolutelyNothing),
			_ => Err(ParseError::InvalidValue(input.to_owned())),
		}
	}
}

fn main() {
	let (mut tx, rx) = std::sync::mpsc::channel::<Ban>();

	let command = command! {
		literal "ban" {
			// Executors can be defined inline for leaf nodes, without requiring a new scope
			argument "name": StringParser {
				argument "reason": BanReasonParser executes |tx: &mut Sender<Ban>, name: String, reason: BanReason| tx.send(Ban { name, reason })

				executes |tx: &mut Sender<Ban>, name: String| tx.send(Ban { name, reason: BanReason::default() })
			}
		}
		literal "error" {
			argument "should": BoolParser executes |_: &mut Sender<Ban>, should: bool| {
				if should {
					command_error!("This is an error")
				} else {
					Ok(())
				}
			}
		}
	};

	// The context can be anything. In this exaxmple, we use a Sender<Ban> to send the information to a receiver, which can be assumed to be the main thread.
	// This is just an example, the context can be anything, including a mutable reference to a struct that holds state.
	assert_matches!(command.execute(&mut tx, "ban you"), Ok(()));
	assert_matches!(command.execute(&mut tx, "ban you absolutely_nothing"), Ok(()));
	assert_matches!(rx.recv(), Ok(Ban { name, reason }) if name == "you" && matches!(reason, BanReason::Nothing));
	assert_matches!(rx.recv(), Ok(Ban { name, reason }) if name == "you" && matches!(reason, BanReason::AbsolutelyNothing));
	assert_matches!(command.execute(&mut tx, "error true"), Err(CommandError::Execution(msg)) if msg == "This is an error");
	assert_matches!(command.execute(&mut tx, "error false"), Ok(()));
}
