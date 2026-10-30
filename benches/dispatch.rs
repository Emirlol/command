#![feature(test, const_default, const_trait_impl)]

extern crate test;

use std::convert::Infallible;

use clap::{
	ColorChoice,
	CommandFactory,
	Parser,
	Subcommand,
};
use command::{
	ArgParser,
	Argument,
	Choice,
	Exec,
	Literal,
	Root,
	StringParser,
	U32Parser,
	command,
};
use test::{
	Bencher,
	black_box,
};

const COMMAND_TREE: Root<
	Choice<
		Literal<"alpha", Exec<fn(&mut u32) -> Result<(), Infallible>>>,
		Choice<
			Literal<"beta", Exec<fn(&mut u32) -> Result<(), Infallible>>>,
			Choice<Literal<"gamma", Exec<fn(&mut u32) -> Result<(), Infallible>>>, Literal<"number", Argument<"value", U32Parser, Exec<fn(&mut u32, u32) -> Result<(), Infallible>>>>>,
		>,
	>,
> = command! {
	literal "alpha" executes |ctx: &mut u32| {
		*ctx += 1;
		Ok::<(), Infallible>(())
		}
	literal "beta" executes |ctx: &mut u32| {
		*ctx += 2;
		Ok::<(), Infallible>(())
	}
	literal "gamma" executes |ctx: &mut u32| {
		*ctx += 3;
		Ok::<(), Infallible>(())
	}
	literal "number" {
		argument "value": U32Parser executes |ctx: &mut u32, value: u32| {
			*ctx += value;
			Ok::<(), Infallible>(())
		}
	}
};

#[derive(Parser)]
#[command(name = "bench", disable_help_flag = true, disable_version_flag = true)]
struct Cli {
	#[command(subcommand)]
	command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
	Alpha,
	Beta,
	Gamma,
	Number { value: u32 },
}

macro_rules! help_tree {
	() => {
		command! {
			literal "user" description "Manage user accounts" {
				literal "ban" description "Block a user from the service" {
					argument "username": StringParser description "Account name to block" executes |_: &mut (), _: String| Ok::<(), Infallible>(())
				}
				literal "unban" description "Restore a user's access" {
					argument "username": StringParser description "Account name to restore" executes |_: &mut (), _: String| Ok::<(), Infallible>(())
				}
			}
			literal "server" description "Select a server" {
				argument "name": StringParser description "Server name" executes |_: &mut (), _: String| Ok::<(), Infallible>(())
			}
			literal "stop" | "halt" description "Stop the service" executes |_: &mut ()| Ok::<(), Infallible>(())
		}
	};
}

#[derive(Parser)]
#[command(name = "app", disable_help_flag = true, disable_help_subcommand = true)]
struct HelpCli {
	#[command(subcommand)]
	command: HelpCommand,
}

#[derive(Subcommand)]
enum HelpCommand {
	#[command(about = "Manage user accounts")]
	User {
		#[command(subcommand)]
		command: UserCommand,
	},
	#[command(about = "Select a server")]
	Server {
		#[arg(value_name = "name", help = "Server name")]
		name: String,
	},
	#[command(alias = "halt", about = "Stop the service")]
	Stop,
}

#[derive(Subcommand)]
enum UserCommand {
	#[command(about = "Block a user from the service")]
	Ban {
		#[arg(value_name = "username", help = "Account name to block")]
		username: String,
	},
	#[command(about = "Restore a user's access")]
	Unban {
		#[arg(value_name = "username", help = "Account name to restore")]
		username: String,
	},
}

fn clap_run(args: &[&str]) -> Result<u32, clap::Error> {
	let parsed = Cli::try_parse_from(std::iter::once("bench").chain(args.iter().copied()))?;
	Ok(match parsed.command {
		CliCommand::Alpha => 1,
		CliCommand::Beta => 2,
		CliCommand::Gamma => 3,
		CliCommand::Number { value } => value,
	})
}

fn direct_tokens(args: &[&str]) -> Result<u32, &'static str> {
	match args {
		["alpha"] => Ok(1),
		["beta"] => Ok(2),
		["gamma"] => Ok(3),
		["number", value] => value.parse().map_err(|_| "invalid number"),
		["number"] => Err("incomplete number"),
		[] => Err("incomplete command"),
		_ => Err("unknown command or trailing input"),
	}
}

fn direct_text(text: &str) -> Result<u32, &'static str> {
	let mut words = text.split_whitespace();
	match words.next() {
		Some("alpha") if words.next().is_none() => Ok(1),
		Some("beta") if words.next().is_none() => Ok(2),
		Some("gamma") if words.next().is_none() => Ok(3),
		Some("number") => match words.next() {
			Some(value) if words.next().is_none() => value.parse().map_err(|_| "invalid number"),
			None => Err("incomplete number"),
			_ => Err("trailing input"),
		},
		None => Err("incomplete command"),
		_ => Err("unknown command"),
	}
}

fn unstyled(text: &str) -> String {
	let mut output = String::with_capacity(text.len());
	let mut chars = text.chars().peekable();
	while let Some(ch) = chars.next() {
		if ch == '\x1b' && chars.peek() == Some(&'[') {
			chars.next();
			for code in chars.by_ref() {
				if code == 'm' {
					break;
				}
			}
		} else {
			output.push(ch);
		}
	}
	output
}

macro_rules! token_bench {
	($library:ident, $direct:ident, $clap:ident, $input:expr, $expected:expr) => {
		#[bench]
		fn $library(b: &mut Bencher) {
			let input: &[&str] = $input;
			let mut ctx = 0u32;
			assert_eq!(direct_tokens(input), Ok($expected));
			assert_eq!(clap_run(input).unwrap(), $expected);
			COMMAND_TREE.execute(&mut ctx, input).unwrap();
			assert_eq!(ctx, $expected);
			b.iter(|| {
				let mut ctx = 0u32;
				let result = COMMAND_TREE.execute(&mut ctx, black_box(input));
				black_box((result, ctx))
			});
		}

		#[bench]
		fn $direct(b: &mut Bencher) {
			let input: &[&str] = $input;
			b.iter(|| black_box(direct_tokens(black_box(input))));
		}

		#[bench]
		fn $clap(b: &mut Bencher) {
			let input: &[&str] = $input;
			b.iter(|| black_box(clap_run(black_box(input))));
		}
	};
}

token_bench!(first_branch_library, first_branch_direct, first_branch_clap, &["alpha"], 1);
token_bench!(later_branch_library, later_branch_direct, later_branch_clap, &["gamma"], 3);
token_bench!(typed_positional_library, typed_positional_direct, typed_positional_clap, &["number", "1234"], 1234);

#[bench]
fn middle_branch_library(b: &mut Bencher) {
	let input = &["beta"][..];
	let mut ctx = 0u32;
	COMMAND_TREE.execute(&mut ctx, input).unwrap();
	assert_eq!(ctx, 2);
	b.iter(|| {
		let mut ctx = 0u32;
		let result = COMMAND_TREE.execute(&mut ctx, black_box(input));
		black_box((result, ctx))
	});
}

#[bench]
fn middle_branch_direct(b: &mut Bencher) {
	b.iter(|| black_box(direct_tokens(black_box(&["beta"]))));
}

#[bench]
fn middle_branch_clap(b: &mut Bencher) {
	b.iter(|| black_box(clap_run(black_box(&["beta"]))));
}

#[bench]
fn missing_branch_library(b: &mut Bencher) {
	let input = &["missing"][..];
	assert!(COMMAND_TREE.execute(&mut 0u32, input).is_err());
	assert!(direct_tokens(input).is_err());
	assert!(clap_run(input).is_err());
	b.iter(|| black_box(COMMAND_TREE.execute(&mut 0u32, black_box(input))));
}

#[bench]
fn missing_branch_direct(b: &mut Bencher) {
	b.iter(|| black_box(direct_tokens(black_box(&["missing"]))));
}

#[bench]
fn missing_branch_clap(b: &mut Bencher) {
	b.iter(|| black_box(clap_run(black_box(&["missing"]))));
}

#[bench]
fn text_input_library(b: &mut Bencher) {
	let input = "gamma";
	let mut ctx = 0u32;
	COMMAND_TREE.execute(&mut ctx, input).unwrap();
	assert_eq!(ctx, 3);
	assert_eq!(direct_text(input), Ok(ctx));
	b.iter(|| {
		let mut ctx = 0u32;
		let result = COMMAND_TREE.execute(&mut ctx, black_box(input));
		black_box((result, ctx))
	});
}

#[bench]
fn text_input_direct(b: &mut Bencher) {
	b.iter(|| black_box(direct_text(black_box("gamma"))));
}

#[bench]
fn typed_argument_direct_parse(b: &mut Bencher) {
	b.iter(|| black_box(black_box("1234").parse::<u32>()));
}

#[bench]
fn typed_argument_library_parser(b: &mut Bencher) {
	let parser = U32Parser;
	let library_value = parser.parse("1234").unwrap();
	let direct_value = "1234".parse::<u32>().unwrap();
	assert_eq!(library_value, direct_value);
	b.iter(|| black_box(parser.parse(black_box("1234"))));
}

#[bench]
fn invalid_argument_library(b: &mut Bencher) {
	let input = &["number", "nope"][..];
	assert!(COMMAND_TREE.execute(&mut 0u32, input).is_err());
	assert!(direct_tokens(input).is_err());
	assert!(clap_run(input).is_err());
	b.iter(|| black_box(COMMAND_TREE.execute(&mut 0u32, black_box(input))));
}

#[bench]
fn invalid_argument_direct(b: &mut Bencher) {
	b.iter(|| black_box(direct_tokens(black_box(&["number", "nope"]))));
}

#[bench]
fn invalid_argument_clap(b: &mut Bencher) {
	b.iter(|| black_box(clap_run(black_box(&["number", "nope"]))));
}

#[bench]
fn incomplete_argument_library(b: &mut Bencher) {
	let input = &["number"][..];
	assert!(COMMAND_TREE.execute(&mut 0u32, input).is_err());
	assert!(direct_tokens(input).is_err());
	assert!(clap_run(input).is_err());
	b.iter(|| black_box(COMMAND_TREE.execute(&mut 0u32, black_box(input))));
}

#[bench]
fn incomplete_argument_direct(b: &mut Bencher) {
	b.iter(|| black_box(direct_tokens(black_box(&["number"]))));
}

#[bench]
fn incomplete_argument_clap(b: &mut Bencher) {
	b.iter(|| black_box(clap_run(black_box(&["number"]))));
}

#[bench]
fn help_root_library(b: &mut Bencher) {
	let commands = help_tree!();
	let output = commands.inspect("").unwrap();
	assert!(output.contains("Usage:"));
	assert!(output.contains("Commands:"));
	assert!(unstyled(&output).contains("<name: String>: Server name"));
	b.iter(|| black_box(commands.inspect(black_box(""))));
}

#[bench]
fn help_nested_library(b: &mut Bencher) {
	let commands = help_tree!();
	let output = commands.inspect("user").unwrap();
	assert!(output.contains("user <command>"));
	assert!(unstyled(&output).contains("ban: Block a user"));
	b.iter(|| black_box(commands.inspect(black_box("user"))));
}

#[bench]
fn help_root_clap_plain(b: &mut Bencher) {
	let mut command = HelpCli::command().color(ColorChoice::Never);
	let _ = command.render_help();
	b.iter(|| black_box(command.render_help().to_string()));
}

#[bench]
fn help_root_clap_ansi(b: &mut Bencher) {
	let mut command = HelpCli::command().color(ColorChoice::Always);
	let _ = command.render_help();
	b.iter(|| black_box(command.render_help().to_string()));
}

#[bench]
fn help_nested_clap_plain(b: &mut Bencher) {
	let mut command = HelpCli::command().color(ColorChoice::Never);
	let user = command.find_subcommand_mut("user").unwrap();
	let _ = user.render_help();
	b.iter(|| black_box(user.render_help().to_string()));
}

#[bench]
fn help_nested_clap_ansi(b: &mut Bencher) {
	let mut command = HelpCli::command().color(ColorChoice::Always);
	let user = command.find_subcommand_mut("user").unwrap();
	let _ = user.render_help();
	b.iter(|| black_box(user.render_help().to_string()));
}
