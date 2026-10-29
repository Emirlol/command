#![allow(incomplete_features)]
#![feature(adt_const_params, unsized_const_params)]

use std::{
	ffi::OsString,
	process::Command,
};

use command::{
	Argument,
	CommandError,
	Exec,
	InputError,
	InputSource,
	IntoInputSource,
	Literal,
	Root,
	StringParser,
};

// A standalone test executable gives each child real, controlled process arguments,
// without libtest flags or changes to the parent process's argument storage.
fn main() {
	if let Ok(source) = std::env::var("COMMAND_INPUT_TEST_SOURCE") {
		match source.as_str() {
			"args" => check(std::env::args()),
			"args_os" => check(std::env::args_os()),
			#[cfg(feature = "argv")]
			"argv" => check(argv::iter()),
			_ => panic!("unknown input source"),
		}
		return;
	}

	let sources = [
		"args",
		"args_os",
		#[cfg(feature = "argv")]
		"argv",
	];
	for source in sources {
		run(source, "valid", &["echo".into(), "".into(), "hello world".into()]);
		run(source, "empty", &[]);
		#[cfg(unix)]
		if source != "args" {
			use std::os::unix::ffi::OsStringExt;
			// Dropping the invalid token would leave a valid command and run the executor.
			run(source, "invalid", &["echo".into(), OsString::from_vec(vec![0xff]), "".into(), "hello world".into()]);
		}
	}
}

fn run(source: &str, case: &str, args: &[OsString]) {
	let output = Command::new(std::env::current_exe().unwrap())
		.env("COMMAND_INPUT_TEST_SOURCE", source)
		.env("COMMAND_INPUT_TEST_CASE", case)
		.args(args)
		.output()
		.unwrap();
	assert!(output.status.success(), "{source}/{case} failed:\n{}", String::from_utf8_lossy(&output.stderr));
}

fn check<Source, I>(input: I)
where
	I: IntoInputSource<Source = Source>,
	CommandError: From<I::Error>,
	Source: InputSource,
{
	let command = Root::new(Literal::<"echo", _>::new(Argument::<"first", _, _>::new(
		StringParser,
		Argument::<"second", _, _>::new(
			StringParser,
			Exec::new(|ctx: &mut Vec<String>, first: String, second: String| {
				ctx.extend([first, second]);
				Ok::<_, CommandError>(())
			}),
		),
	)));
	let mut calls = Vec::new();
	let result = command.execute(&mut calls, input);
	match std::env::var("COMMAND_INPUT_TEST_CASE").unwrap().as_str() {
		"valid" => {
			result.unwrap();
			assert_eq!(calls, ["", "hello world"]);
		}
		"empty" => {
			assert!(matches!(result, Err(CommandError::IncompleteCommand)));
			assert!(calls.is_empty());
		}
		"invalid" => {
			let Err(CommandError::InputError(InputError::InvalidUtf8(value))) = result else {
				panic!("expected InvalidUtf8, got {result:?}");
			};
			#[cfg(unix)]
			{
				use std::os::unix::ffi::OsStrExt;
				assert_eq!(value.as_os_str().as_bytes(), &[0xff]);
			}
			assert!(calls.is_empty(), "invalid input must not execute a command");
		}
		_ => panic!("unknown test case"),
	}
}
