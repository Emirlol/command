#![allow(incomplete_features)]
#![feature(adt_const_params, unsized_const_params)]

use command::{
	Argument,
	Choice,
	CommandError,
	Exec,
	I32Parser,
	Input,
	InputSource,
	IntoInputSource,
	Literal,
	Nil,
	Root,
	StringParser,
};

fn assert_dispatch(input: impl IntoInputSource) {
	let command = Root::new(Choice::new(
		Literal::<"stop", _>::new(Nil),
		Literal::<"echo", _>::new(Argument::<"value", _, _>::new(
			StringParser,
			Exec::new(|ctx: &mut Vec<String>, value: String| {
				ctx.push(value);
				Ok::<_, CommandError>(())
			}),
		)),
	));
	let mut calls = Vec::new();
	command.execute(&mut calls, input).unwrap();
	assert_eq!(calls, ["hello"]);
}

#[test]
fn supported_sources_execute_after_restoring_a_literal_mismatch() {
	let text = " \techo\u{2003}hello\n".to_owned();
	assert_dispatch(text.as_str());
	assert_dispatch(&text);
	assert_dispatch(text);
	let array = ["echo", "hello"];
	assert_dispatch(array);
	assert_dispatch(&array);
	assert_dispatch(array.as_slice());
	assert_dispatch(vec!["echo", "hello"]);
	let owned = array.map(str::to_owned);
	assert_dispatch(owned.as_slice());
	assert_dispatch(owned);
	let owned = vec!["echo".to_owned(), "hello".to_owned()];
	assert_dispatch(&owned);
	assert_dispatch(owned);
}

#[test]
fn token_sources_preserve_empty_and_whitespace_containing_arguments() {
	let command = Root::new(Argument::<"first", _, _>::new(
		StringParser,
		Argument::<"second", _, _>::new(
			StringParser,
			Exec::new(|ctx: &mut Vec<String>, first: String, second: String| {
				ctx.extend([first, second]);
				Ok::<_, CommandError>(())
			}),
		),
	));
	let mut calls = Vec::new();
	command.execute(&mut calls, ["", "hello world"]).unwrap();
	assert_eq!(calls, ["", "hello world"]);
}

#[test]
fn failed_argument_parser_restores_token_for_next_branch() {
	let command = Root::new(Choice::new(
		Argument::<"number", _, _>::new(
			I32Parser,
			Exec::new(|_: &mut Vec<String>, _: i32| -> Result<(), CommandError> { panic!("unexpected integer executor") }),
		),
		Argument::<"text", _, _>::new(
			StringParser,
			Exec::new(|ctx: &mut Vec<String>, text: String| {
				ctx.push(text);
				Ok::<_, CommandError>(())
			}),
		),
	));
	let mut calls = Vec::new();
	command.execute(&mut calls, vec!["hello world".to_owned()]).unwrap();
	assert_eq!(calls, ["hello world"]);
}

fn assert_empty(input: impl IntoInputSource) {
	let command = Root::new(Nil);
	assert!(matches!(command.execute_no_context(input), Err(CommandError::IncompleteCommand)));
}

#[test]
fn empty_sources_report_incomplete_commands() {
	assert_empty("");
	assert_empty(" \t\u{2003}\n");
	assert_empty(String::new());
	assert_empty(&String::new());
	let empty: [&str; 0] = [];
	assert_empty(empty);
	assert_empty(empty.as_slice());
	assert_empty(Vec::<String>::new());
}

fn assert_cursor(source: impl InputSource) {
	assert_eq!(source.remaining_tokens(source.len()), None);
	assert_eq!(source.remaining_tokens(source.len() + 1), None);
	let mut input = Input::new(source);
	let start = input.checkpoint();
	assert_eq!(input.next_token(), Some(""));
	assert_eq!(input.remaining_tokens(), Some(vec!["hello world"]));
	assert_eq!(input.next_token(), Some("hello world"));
	assert!(!input.has_remaining());
	assert_eq!(input.remaining_tokens(), None);
	assert_eq!(input.next_token(), None);
	assert_eq!(input.next_token(), None);
	input.restore(start);
	assert!(input.has_remaining());
	assert_eq!(input.next_token(), Some(""));
}

#[test]
fn cursors_restore_after_exhaustion() {
	let tokens = ["", "hello world"];
	assert_cursor(tokens);
	assert_cursor(tokens.as_slice());
	assert_cursor(tokens.map(str::to_owned).to_vec());
}

#[test]
fn empty_trailing_argument_is_not_end_of_input() {
	let command = Root::new(Literal::<"stop", _>::new(Exec::new(|_: &mut ()| -> Result<(), CommandError> {
		panic!("executor must not run with trailing input")
	})));
	assert!(matches!(command.execute_no_context(["stop", ""]), Err(CommandError::TrailingInput(value)) if value.is_empty()));
}
