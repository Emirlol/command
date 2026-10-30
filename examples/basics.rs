use std::{
	assert_matches,
	convert::Infallible,
};

// These imports are normally not required, but since the example is within the same project as the main lib, the macro impl generates crate:: prefixed paths which break in this example
use command::{
	Argument,
	Choice,
	CommandError,
	Exec,
	Literal,
	Root,
	StringParser,
	WithExec,
};
use macros::command;

fn main() {
	let command/*: Root< // The root node with the convenience methods to execute and inspect the command tree
		Choice< // A choice node, which will try to parse the input with the first child, and if it fails, will try the second child
			Literal< // A literal node that will match a literal string input
				"hello", // The literal string to match
				WithExec< // A node that has both an executor and children, meaning the children of the node in this node are optional and may be omitted (in which case, this WithExec's executor will run)
					Argument< // An Argument node that will parse an argument with a parser type
						"name", // The name of the argument being expected, for help output & general convenience
						StringParser, // The parser type to use for this argument, which will parse the input into a value of the type that the parser returns. Custom argument parsers can be defined by implementing the `ArgParser` trait.
						Exec< // Executor wrapper node to not pollute the type system for functions for any node trait impls
							// The executor function of this node. For executors, the first parameter is always the context for the command tree, which may hold state, mpsc channels, or anything else you want to pass into the executor.
							// The context will be passes in at the root, and will be available on any executor node.
							// The rest of the parameters are the arguments that have been parsed from the parent `Argument` nodes in the execution path, in the order of the nodes in the tree.
							// This means a node has access to the arguments of its parent nodes, but not its children.
							fn(&mut (), String) -> Result<(), Infallible>,
						>,
					>,
					fn(&mut ()) -> Result<(), Infallible> // The executor of this `WithExec`
				>,
			>,
			Choice< // Choice nodes can be chained to create a tree of sibling nodes
				Literal<"hello2", Argument<"name", StringParser, Exec<fn(&mut (), String) -> Result<(), Infallible>>>>,
				Literal<
					"test",
					Argument<
						"arg1",
						StringParser,
						WithExec<
							Argument<"arg2", StringParser, Argument<"arg3", StringParser, Exec<fn(&mut (), String, String, String) -> Result<(), Infallible>>>>,
							fn(&mut (), String) -> Result<(), Infallible>,
						>,
					>,
				>,
			>,
		>,
	> */= command! {
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

	assert_matches!(command.execute(&mut (), "hello there"), Ok(()));
	assert_matches!(command.execute(&mut (), "hello".to_string()), Ok(()));
	assert_matches!(command.execute(&mut (), "hello2 function_test"), Ok(()));
	// The input doesn't have to be a single string, it can also be an array/slice/vector of strings or command-line args obtained from std::env::args() (which will skip the first arg, which is the program name)
	assert_matches!(command.execute(&mut (), ["test", "hi", "there", "test"]), Ok(()));
	assert_matches!(command.execute(&mut (), "test hi there"), Err(CommandError::IncompleteCommand)); // An executor was not defined for arg2 node, it will return an Err
	assert_matches!(command.execute(&mut (), "test hi"), Ok(()));
}

fn test_print(_: &mut (), name: String) -> Result<(), Infallible> {
	println!("Hello {name}!");
	Ok(())
}
