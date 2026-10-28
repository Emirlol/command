use std::{
	collections::{
		BTreeMap,
		BTreeSet,
	},
	convert::Infallible,
	io::{
		self,
		BufRead,
		Write,
	},
};

// The macro expansion uses these crate types directly in this in-repository example.
use command::{
	Argument,
	Choice,
	Exec,
	Literal,
	Root,
	StringParser,
	U32Parser,
};
use macros::command;

#[derive(Default)]
struct DemoState {
	server_running: bool,
	users: BTreeSet<String>,
	settings: BTreeMap<String, String>,
}

/// An interactive repl to see the command tree in action. Run with the `ansi` feature toggled on for colors, like `cargo run --example repl --features ansi`.
fn main() -> io::Result<()> {
	let command = command! {
		literal "hello" description "Greet someone" {
			argument "name": StringParser description "The name to greet" executes |_: &mut DemoState, name: String| {
				println!("Hello, {name}!");
				Ok::<_, Infallible>(())
			}
		}
		literal "echo" description "Print one word" {
			argument "text": StringParser description "Text to print" executes |_: &mut DemoState, text: String| {
				println!("{text}");
				Ok::<_, Infallible>(())
			}
		}
		literal "math" description "Integer arithmetic" {
			literal "add" description "Add two unsigned integers" {
				argument "left": U32Parser description "First number" {
					argument "right": U32Parser description "Second number" executes |_: &mut DemoState, left: u32, right: u32| {
						println!("{} + {} = {}", left, right, left + right);
						Ok::<_, Infallible>(())
					}
				}
			}
			literal "multiply" description "Multiply two unsigned integers" {
				argument "left": U32Parser description "First number" {
					argument "right": U32Parser description "Second number" executes |_: &mut DemoState, left: u32, right: u32| {
						println!("{} * {} = {}", left, right, left * right);
						Ok::<_, Infallible>(())
					}
				}
			}
		}
		literal "server" description "Manage the demo server" {
			literal "start" description "Start the server" executes |state: &mut DemoState| {
				state.server_running = true;
				println!("Server started.");
				Ok::<_, Infallible>(())
			}
			literal "stop" description "Stop the server" executes |state: &mut DemoState| {
				state.server_running = false;
				println!("Server stopped.");
				Ok::<_, Infallible>(())
			}
			literal "status" description "Show server status" executes |state: &mut DemoState| {
				println!("Server is {}.", if state.server_running { "running" } else { "stopped" });
				Ok::<_, Infallible>(())
			}
		}
		literal "user" description "Manage demo users" {
			literal "add" description "Add a user" {
				argument "name": StringParser description "The user name" executes |state: &mut DemoState, name: String| {
					if state.users.insert(name.clone()) {
						println!("Added user {name}.");
					} else {
						println!("User {name} already exists.");
					}
					Ok::<_, Infallible>(())
				}
			}
			literal "remove" description "Remove a user" {
				argument "name": StringParser description "The user name" executes |state: &mut DemoState, name: String| {
					if state.users.remove(&name) {
						println!("Removed user {name}.");
					} else {
						println!("User {name} does not exist.");
					}
					Ok::<_, Infallible>(())
				}
			}
			literal "list" description "List users" executes |state: &mut DemoState| {
				if state.users.is_empty() {
					println!("No users.");
				} else {
					println!("Users: {}", state.users.iter().cloned().collect::<Vec<_>>().join(", "));
				}
				Ok::<_, Infallible>(())
			}
		}
		literal "config" description "Read and change in-memory settings" {
			literal "set" description "Set a key to a value" {
				argument "key": StringParser description "Setting name" {
					argument "value": StringParser description "Setting value" executes |state: &mut DemoState, key: String, value: String| {
						state.settings.insert(key.clone(), value.clone());
						println!("Set {key} = {value}");
						Ok::<_, Infallible>(())
					}
				}
			}
			literal "get" description "Read a setting" {
				argument "key": StringParser description "Setting name" executes |state: &mut DemoState, key: String| {
					match state.settings.get(&key) {
						Some(value) => println!("{key} = {value}"),
						None => println!("No value set for {key}."),
					}
					Ok::<_, Infallible>(())
				}
			}
			literal "list" description "List settings" executes |state: &mut DemoState| {
				if state.settings.is_empty() {
					println!("No settings.");
				} else {
					for (key, value) in &state.settings {
						println!("{key} = {value}");
					}
				}
				Ok::<_, Infallible>(())
			}
		}
	};

	let stdin = io::stdin();
	let mut state = DemoState::default();
	let mut line = String::new();
	println!("Interactive command demo. Enter `help` to explore; press Ctrl+C to quit.");

	loop {
		print!("command> ");
		io::stdout().flush()?;
		line.clear();
		match stdin.lock().read_line(&mut line) {
			Ok(0) => break,
			Ok(_) => {}
			Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
			Err(error) => return Err(error),
		}

		let input = line.trim();
		if input.is_empty() {
			continue;
		}
		match command.run(&mut state, input) {
			Ok(Some(output)) => print!("{output}"),
			Ok(None) => {}
			Err(error) => eprintln!("{error}"),
		}
	}

	Ok(())
}
