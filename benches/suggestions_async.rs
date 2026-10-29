#![feature(test, const_default, const_trait_impl)]

extern crate test;

use std::{
	borrow::Cow,
	future::Future,
	task::{
		Context,
		Poll,
		Waker,
	},
};

use command::{
	Argument,
	BoolSuggester,
	Choice,
	ConstSuggester,
	Literal,
	Nil,
	Root,
	StringParser,
};
use test::{
	Bencher,
	black_box,
};

type UserIdSuggester = ConstSuggester<
	{
		&[
			"usr-0001", "usr-0002", "usr-0003", "usr-0004", "usr-0005", "usr-0006", "usr-0007", "usr-0008", "usr-0009", "usr-0010", "usr-0011", "usr-0012", "usr-0013", "usr-0014", "usr-0015",
			"usr-0016",
		]
	},
>;

type SuggestionTree = Root<
	Choice<
		Literal<"user", Argument<"id", StringParser, Nil, "", UserIdSuggester>>,
		Choice<Literal<"status", Argument<"value", StringParser, Nil, "", BoolSuggester>>, Choice<Literal<"list", Nil>, Nil>>,
	>,
>;

const COMMAND_TREE: SuggestionTree = SuggestionTree::default();

fn run_ready<F: Future>(future: F) -> F::Output {
	let mut future = std::pin::pin!(future);
	let mut context = Context::from_waker(Waker::noop());
	match future.as_mut().poll(&mut context) {
		Poll::Ready(output) => output,
		Poll::Pending => panic!("benchmark suggestion future unexpectedly yielded"),
	}
}

fn assert_suggestions(actual: &[Cow<'_, str>], expected: &[&str]) {
	assert_eq!(actual.iter().map(Cow::as_ref).collect::<Vec<_>>(), expected);
}

macro_rules! suggestion_bench {
	($tree:expr, $name:ident, $input:expr, $expected:expr) => {
		#[bench]
		fn $name(b: &mut Bencher) {
			let tree: SuggestionTree = $tree;
			let context = ();
			let input = $input;
			let expected: &[&str] = $expected;
			let result = run_ready(tree.suggest(&context, input)).unwrap();
			assert_suggestions(&result, expected);
			b.iter(|| black_box(run_ready(tree.suggest(&context, black_box(input))).unwrap()));
		}
	};
}

suggestion_bench!(COMMAND_TREE, root_literals, &["u"], &["user"]);
suggestion_bench!(
	COMMAND_TREE,
	nested_provider,
	&["user", "usr-000"],
	&["usr-0001", "usr-0002", "usr-0003", "usr-0004", "usr-0005", "usr-0006", "usr-0007", "usr-0008", "usr-0009"]
);
suggestion_bench!(COMMAND_TREE, boolean_provider, &["status", "t"], &["true"]);
suggestion_bench!(COMMAND_TREE, no_matches, &["user", "missing"], &[]);
