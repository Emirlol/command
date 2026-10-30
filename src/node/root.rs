use crate::{
	CommandError,
	DispatchError,
	ExecutableNode,
	Input,
	InputSource,
	InspectableNode,
	InspectionError,
	IntoInputSource,
	NodeInspector,
};

/// The root of a command tree.
pub struct Root<Children> {
	children: Children,
}

const impl<Children> Root<Children> {
	#[inline(always)]
	pub fn new(children: Children) -> Self {
		Self { children }
	}
}

const impl<Children> Default for Root<Children>
where
	Children: [const] Default,
{
	#[inline(always)]
	fn default() -> Self {
		Self::new(Children::default())
	}
}

impl<Children> Root<Children> {
	/// Executes input, returning formatted help text when the input is the built-in `help` command.
	///
	/// Normal command executors still control their own side effects and return `Ok(None)` here.
	/// `run` itself does not write to stdout or stderr.
	#[inline(always)]
	pub fn run<Ctx, Source>(&self, ctx: &mut Ctx, input: impl IntoInputSource<Source = Source>) -> Result<Option<String>, CommandError>
	where
		Source: InputSource,
		Children: ExecutableNode<Ctx, (), Source> + InspectableNode<Source>,
	{
		let mut input = Input::new(input.into_input_source()?);
		if let Some(token) = input.peek() {
			if token == "help" {
				input.advance(1); // The help command can match sub-commands and show the help for those layers, so we need to remove `help` from the input before inspecting the tree.
				let mut inspector = NodeInspector::new();
				self.children.inspect(&mut inspector, &mut input).map_err(|error| error.into_command_error(&input))?;
				let path = input.remaining_tokens_at(1).map(|tokens| tokens.into_iter().collect::<Vec<_>>()).unwrap_or_default();
				let output = crate::inspect::format_help_output(&inspector.output, &path);
				Ok(Some(output))
			} else {
				match self.children.execute(ctx, &mut input, ()) {
					Ok(()) => Ok(None),
					Err(DispatchError::Recoverable(error, _)) => Err(error.into_command_error(&input)),
					Err(DispatchError::Fatal(error)) => Err(error),
				}
			}
		} else {
			Err(CommandError::IncompleteCommand)
		}
	}

	/// Executes the command tree with the given context and input source.
	///
	/// If there's no context to pass to the executor, you can alternatively use [`execute_no_context`] which will use `()` as the context type.
	///
	/// See [IntoInputSource] for supported input types.
	#[inline(always)]
	pub fn execute<Ctx, Source>(&self, ctx: &mut Ctx, input: impl IntoInputSource<Source = Source>) -> Result<(), CommandError>
	where
		Source: InputSource,
		Children: ExecutableNode<Ctx, (), Source>,
	{
		let mut input = Input::new(input.into_input_source()?);
		match self.children.execute(ctx, &mut input, ()) {
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
		Children: ExecutableNode<(), (), Source>,
	{
		self.execute(&mut (), input)
	}

	#[inline(always)]
	pub fn inspect<Source>(&self, input: impl IntoInputSource<Source = Source>) -> Result<String, InspectionError>
	where
		Source: InputSource,
		Children: InspectableNode<Source>,
	{
		let mut input = Input::new(input.into_input_source()?);

		let mut inspector = NodeInspector::new();
		self.children.inspect(&mut inspector, &mut input)?;
		let path = input.remaining_tokens_at(0).map(|tokens| tokens.into_iter().collect::<Vec<_>>()).unwrap_or_default();
		Ok(crate::inspect::format_help_output(&inspector.output, &path))
	}
}

#[cfg(test)]
mod tests {
	use std::convert::Infallible;

	use macros::command;

	use crate::{
		BoolParser,
		StringParser,
	};

	fn plain_output(output: String) -> String {
		#[cfg(feature = "ansi")]
		{
			let mut plain = String::with_capacity(output.len());
			let mut chars = output.chars().peekable();
			while let Some(ch) = chars.next() {
				if ch == '\x1b' && chars.peek() == Some(&'[') {
					chars.next();
					for code in chars.by_ref() {
						if code == 'm' {
							break;
						}
					}
				} else {
					plain.push(ch);
				}
			}
			plain
		}
		#[cfg(not(feature = "ansi"))]
		{
			output
		}
	}

	#[test]
	fn help_at_root_lists_only_first_layer() {
		let commands = command! {
			literal "ban" description "Ban a user" {
				argument "user": StringParser description "The user to ban" executes |_: &mut (), _: String| Ok::<(), Infallible>(())
			}
			literal "kick" description "Kick a user" executes |_: &mut ()| Ok::<(), Infallible>(())
		};

		assert_eq!(
			plain_output(commands.inspect("").unwrap()),
			"Usage:\n  <command> [<arguments>...]\n\nCommands:\n  ban: Ban a user\n    <user: String>: The user to ban\n  kick: Kick a user\n\n",
		);
	}

	#[test]
	fn description_blocks_are_parsed_for_literal_and_argument_nodes() {
		let commands = command! {
			literal "find" {
				description {
					concat!("Find", " a record")
				}
				argument "query": StringParser {
					description {
						concat!("Text", " to search for")
					}
					executes |_: &mut (), _: String| Ok::<(), Infallible>(())
				}
			}
		};

		let output = plain_output(commands.inspect("").unwrap());
		assert!(output.contains("find: Find a record"));
		assert!(output.contains("<query: String>: Text to search for"));
	}

	#[test]
	fn help_keeps_nested_arguments_under_their_command() {
		let commands = command! {
			literal "ban" description "Description" {
				argument "user": StringParser description "Description" {
					argument "reason": StringParser description "Description" executes |_: &mut (), _: String, _: String| Ok::<(), Infallible>(())
				}
			}
		};

		assert_eq!(
			plain_output(commands.inspect("ban").unwrap()),
			"Usage:\n  ban <user: String> <reason: String>\n\nCommands:\n  ban: Description\n    <user: String>: Description\n      <reason: String>: Description\n\n",
		);
	}

	#[test]
	fn help_for_a_command_lists_its_next_layer() {
		let commands = command! {
			literal "run" | "stop" description "Choose an action" {
				literal "server" description "Server target" executes |_: &mut ()| Ok::<(), Infallible>(())
				literal "client" description "Client target" executes |_: &mut ()| Ok::<(), Infallible>(())
			}
		};

		assert_eq!(
			plain_output(commands.inspect("run").unwrap()),
			"Usage:\n  run <command>\n\nCommands:\n  run | stop: Choose an action\n    server: Server target\n    client: Client target\n\n",
		);
	}

	#[test]
	fn help_at_an_invalid_argument_shows_the_argument_node() {
		let commands = command! {
			literal "set" description "Set a value" {
				argument "enabled": BoolParser description "Whether to enable it" executes |_: &mut (), _: bool| Ok::<(), Infallible>(())
			}
		};

		assert_eq!(
			plain_output(commands.inspect("set invalid").unwrap()),
			"Usage:\n  set <enabled: bool>\n\nCommands:\n  set: Set a value\n    <enabled: bool>: Whether to enable it\n\n",
		);
	}

	#[test]
	fn help_lists_a_leaf_command_only_once() {
		let commands = command! {
			literal "user" {
				literal "list" executes |_: &mut ()| Ok::<(), Infallible>(())
			}
		};

		let output = plain_output(commands.inspect("user list").unwrap());
		assert_eq!(output.matches("list:").count(), 1);
		assert!(output.starts_with("Usage:\n  user list\n"));
	}

	#[test]
	fn help_styling_matches_the_ansi_feature() {
		let commands = command! {
			literal "find" description "Find a record" {
				argument "query": StringParser description "Search text" executes |_: &mut (), _: String| Ok::<(), Infallible>(())
			}
		};
		let output = commands.inspect("").unwrap();

		#[cfg(feature = "ansi")]
		{
			assert!(output.contains("\x1b[1;32mUsage:\x1b[0m"));
			assert!(output.contains("\x1b[1;34mfind\x1b[0m"));
			assert!(output.contains("\x1b[36m<query: \x1b[0m\x1b[1;33mString\x1b[0m\x1b[36m>\x1b[0m"));
			let command_help = commands.inspect("find").unwrap();
			assert!(command_help.contains("\x1b[1;32mfind \x1b[0m\x1b[36m<query: \x1b[0m\x1b[1;33mString\x1b[0m\x1b[36m>\x1b[0m"));
		}
		#[cfg(not(feature = "ansi"))]
		assert!(!output.contains('\x1b'));
	}

	#[test]
	fn run_returns_help_output_and_no_output_for_executed_commands() {
		let commands = command! {
			literal "ping" executes |_: &mut ()| Ok::<(), Infallible>(())
		};

		let help = commands.run(&mut (), "help").unwrap().unwrap();
		assert!(plain_output(help).contains("ping: No description provided."));
		assert_eq!(commands.run(&mut (), "ping").unwrap(), None);
	}
}
