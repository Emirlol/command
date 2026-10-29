//! Token suggestions for command trees.

use std::borrow::Cow;
#[cfg(feature = "async-suggestions")]
use std::future::Future;

use crate::{
	InputSource,
	SliceInputSource,
	SliceLike,
	Unknown,
};

/// Supplies values for an argument at the current token position.
///
/// The default implementation used by [`Argument`](crate::Argument) returns no values.
/// Implement this trait when an argument has a finite or context-dependent set of useful completions.
#[cfg(not(feature = "async-suggestions"))]
pub trait SuggestionProvider<Ctx> {
	fn suggest<'c>(&'c self, context: &'c Ctx, prefix: &str) -> Vec<Cow<'c, str>>;
}

/// Supplies values for an argument at the current token position.
///
/// The default implementation used by [`Argument`](crate::Argument) returns no values.
/// Implement this trait when an argument has a finite or context-dependent set of useful completions.
///
/// The returned future must be `Send` so suggestion requests can be moved between async runtime threads.
#[cfg(feature = "async-suggestions")]
pub trait SuggestionProvider<Ctx>: Send + Sync
where
	Ctx: Sync,
{
	fn suggest<'c>(&'c self, context: &'c Ctx, prefix: &str) -> impl Future<Output = Vec<Cow<'c, str>>> + Send;
}

/// Traverses a command node to collect completions for the current token.
///
/// `completed` contains the exact tokens before the token being completed;
/// `prefix` is the current token, or an empty string when suggesting a new token.
///
/// Results are candidate token strings only.
#[cfg(not(feature = "async-suggestions"))]
pub trait SuggestibleNode<Ctx> {
	fn suggest<'c>(&'c self, context: &'c Ctx, completed: &[&str], prefix: &str, output: &mut Vec<Cow<'c, str>>);
}

/// Traverses a command node to collect completions for the current token.
///
/// `completed` contains the exact tokens before the token being completed;
/// `prefix` is the current token, or an empty string when suggesting a new token.
///
/// Results are candidate token strings only.
#[cfg(feature = "async-suggestions")]
pub trait SuggestibleNode<Ctx>: Sync
where
	Ctx: Sync,
{
	fn suggest<'c>(&'c self, context: &'c Ctx, completed: &[&str], prefix: &str, output: &mut Vec<Cow<'c, str>>) -> impl Future<Output = ()> + Send;
}

/// A suggester that contributes no argument suggestions.
#[derive(Clone, Copy, Debug)]
pub struct NoSuggestions;

const impl Default for NoSuggestions {
	#[inline(always)]
	fn default() -> Self {
		Self
	}
}

#[cfg(not(feature = "async-suggestions"))]
impl<Ctx> SuggestionProvider<Ctx> for NoSuggestions {
	#[inline(always)]
	fn suggest<'c>(&'c self, _context: &'c Ctx, _prefix: &str) -> Vec<Cow<'c, str>> {
		Vec::new()
	}
}

#[cfg(feature = "async-suggestions")]
impl<Ctx> SuggestionProvider<Ctx> for NoSuggestions
where
	Ctx: Sync,
{
	#[inline(always)]
	fn suggest<'c>(&'c self, _context: &'c Ctx, _prefix: &str) -> impl Future<Output = Vec<Cow<'c, str>>> + Send {
		async { Vec::new() }
	}
}

/// A suggester that contributes a fixed set of argument suggestions.
pub struct ConstSuggester<const SUGGESTIONS: &'static [&'static str]>;

const impl<const SUGGESTIONS: &'static [&'static str]> Default for ConstSuggester<SUGGESTIONS> {
	#[inline(always)]
	fn default() -> Self {
		Self
	}
}

#[cfg(not(feature = "async-suggestions"))]
impl<const SUGGESTIONS: &'static [&'static str], Ctx> SuggestionProvider<Ctx> for ConstSuggester<SUGGESTIONS>
where
	Ctx: Sync,
{
	#[inline(always)]
	fn suggest<'c>(&'c self, _context: &'c Ctx, prefix: &str) -> Vec<Cow<'c, str>> {
		SUGGESTIONS.iter().filter(|candidate| candidate.starts_with(prefix)).map(|s| Cow::Borrowed(*s)).collect()
	}
}

#[cfg(feature = "async-suggestions")]
impl<const SUGGESTIONS: &'static [&'static str], Ctx> SuggestionProvider<Ctx> for ConstSuggester<SUGGESTIONS>
where
	Ctx: Sync,
{
	#[inline(always)]
	fn suggest<'c>(&'c self, _context: &'c Ctx, prefix: &str) -> impl Future<Output = Vec<Cow<'c, str>>> + Send {
		async move { SUGGESTIONS.iter().filter(|candidate| candidate.starts_with(prefix)).map(|s| Cow::Borrowed(*s)).collect() }
	}
}

/// A suggestion provider that contributes the values "true" and "false" for a boolean argument.
pub type BoolSuggester = ConstSuggester<{ &["true", "false"] }>;

/// A suggestion provider that queries the filesystem for completions of a path argument.
///
/// This isn't filtered against any relative components, so the suggestions may include any valid path on the device.
/// If you want to restrict the suggestions to a specific directory or validate the directories, you can implement your own `SuggestionProvider` that filters the results of this one (or copy-paste the impl and tweak it as you desire).
///
/// This also doesn't do async I/O and has no impl when `async-suggestions` is enabled, because doing so would require bringing in new dependencies, which I don't want to do.
/// If you really need async I/O for path suggestions, you can implement your own `SuggestionProvider` that does so.
/// Alternatively, you can wrap the sync impl in an async block, but that will block the async runtime thread while the I/O is happening, which may not be desirable.
pub struct PathSuggester;

const impl Default for PathSuggester {
	#[inline(always)]
	fn default() -> Self {
		Self
	}
}

#[cfg(not(feature = "async-suggestions"))]
impl<Ctx> SuggestionProvider<Ctx> for PathSuggester {
	#[inline(always)]
	fn suggest<'c>(&'c self, _context: &'c Ctx, prefix: &str) -> Vec<Cow<'c, str>> {
		use std::path::Path;

		if prefix.is_empty() {
			let Ok(current_dir) = std::env::current_dir() else {
				return Vec::new();
			};
			let Ok(iter) = current_dir.read_dir() else {
				return Vec::new();
			};
			iter.filter_map(|entry| entry.ok()).filter_map(|entry| entry.file_name().into_string().ok()).map(Cow::Owned).collect()
		} else {
			let path = Path::new(prefix);
			if path.exists() {
				if path.is_dir() {
					if prefix.ends_with(std::path::SEPARATORS) {
						// dir/ is complete, list all entries
						let Ok(iter) = path.read_dir() else {
							return Vec::new();
						};
						iter.filter_map(|entry| entry.ok()).filter_map(|entry| entry.file_name().into_string().ok()).map(Cow::Owned).collect()
					} else {
						// Suggest completing the dir into dir/ with the separator at the end, the next suggestion should list that dir's entries
						vec![Cow::Borrowed(std::path::MAIN_SEPARATOR_STR)]
					}
				} else {
					// Not a dir, suggest all entries in the parent dir that start with the file name
					let Some(parent) = path.parent() else {
						return Vec::new();
					};
					let Ok(iter) = parent.read_dir() else {
						return Vec::new();
					};
					let Some(path_file_name) = path.file_name() else {
						return Vec::new();
					};
					let path_bytes = path_file_name.as_encoded_bytes();

					iter.filter_map(|entry| entry.ok())
						.map(|entry| entry.file_name())
						.filter(|name| name.as_encoded_bytes().starts_with(path_bytes))
						.filter_map(|name| name.into_string().ok())
						.map(Cow::Owned)
						.collect()
				}
			} else if let Some(parent) = path.parent()
				&& parent.exists()
				&& parent.is_dir()
			{
				// The path doesn't exist, but the parent dir does, suggest all entries in the parent dir that start with the file name
				let Ok(iter) = parent.read_dir() else {
					return Vec::new();
				};
				let Some(path_file_name) = path.file_name() else {
					return Vec::new();
				};
				let path_bytes = path_file_name.as_encoded_bytes();

				iter.filter_map(|entry| entry.ok())
					.map(|entry| entry.file_name())
					.filter(|name| name.as_encoded_bytes().starts_with(path_bytes))
					.filter_map(|name| name.into_string().ok())
					.map(Cow::Owned)
					.collect()
			} else {
				Vec::new()
			}
		}
	}
}

#[cfg(not(feature = "async-suggestions"))]
pub trait SuggestDispatch<S>
where
	S: InputSource,
{
	fn suggest<'c, Ctx, Children>(source: &S, context: &'c Ctx, children: &'c Children) -> Vec<Cow<'c, str>>
	where
		Children: SuggestibleNode<Ctx>;
}

#[cfg(not(feature = "async-suggestions"))]
impl<S> SuggestDispatch<S> for Unknown
where
	S: InputSource<Type = Unknown>,
{
	#[inline(always)]
	fn suggest<'c, Ctx, Children>(source: &S, context: &'c Ctx, children: &'c Children) -> Vec<Cow<'c, str>>
	where
		Children: SuggestibleNode<Ctx>,
	{
		let prefix = if source.is_empty() { "" } else { source.get(source.len() - 1).unwrap_or("") };
		let completed = if source.is_empty() {
			Vec::new()
		} else {
			source.remaining_tokens(0).map(|i| i.into_iter().take(source.len() - 1).collect::<Vec<_>>()).unwrap_or_default()
		};
		let mut suggestions = Vec::new();
		children.suggest(context, &completed, prefix, &mut suggestions);
		suggestions
	}
}

#[cfg(not(feature = "async-suggestions"))]
impl<S> SuggestDispatch<S> for SliceLike
where
	S: SliceInputSource,
{
	#[inline(always)]
	fn suggest<'c, Ctx, Children>(source: &S, context: &'c Ctx, children: &'c Children) -> Vec<Cow<'c, str>>
	where
		Children: SuggestibleNode<Ctx>,
	{
		let mut suggestions = Vec::new();
		if let Some((prefix, completed)) = source.as_slice().split_last() {
			children.suggest(context, completed, *prefix, &mut suggestions);
		} else {
			children.suggest(context, &[], "", &mut suggestions);
		}
		suggestions
	}
}

#[cfg(feature = "async-suggestions")]
pub trait SuggestDispatch<S>
where
	S: InputSource,
{
	fn suggest<'c, Ctx, Children>(source: &S, context: &'c Ctx, children: &'c Children) -> impl Future<Output = Vec<Cow<'c, str>>> + Send
	where
		Children: SuggestibleNode<Ctx>,
		Ctx: Sync;
}

#[cfg(feature = "async-suggestions")]
impl<S> SuggestDispatch<S> for Unknown
where
	S: InputSource<Type = Unknown>,
{
	#[inline(always)]
	fn suggest<'c, Ctx, Children>(source: &S, context: &'c Ctx, children: &'c Children) -> impl Future<Output = Vec<Cow<'c, str>>> + Send
	where
		Children: SuggestibleNode<Ctx>,
		Ctx: Sync,
	{
		let prefix = if source.is_empty() { "" } else { source.get(source.len() - 1).unwrap_or("") };
		let completed = if source.is_empty() {
			Vec::new()
		} else {
			source.remaining_tokens(0).map(|i| i.into_iter().take(source.len() - 1).collect::<Vec<_>>()).unwrap_or_default()
		};
		async move {
			let mut suggestions = Vec::new();
			children.suggest(context, &completed, prefix, &mut suggestions).await;
			suggestions
		}
	}
}

#[cfg(feature = "async-suggestions")]
impl<S> SuggestDispatch<S> for SliceLike
where
	S: SliceInputSource + Sync,
{
	#[inline(always)]
	fn suggest<'c, Ctx, Children>(source: &S, context: &'c Ctx, children: &'c Children) -> impl Future<Output = Vec<Cow<'c, str>>> + Send
	where
		Children: SuggestibleNode<Ctx>,
		Ctx: Sync,
	{
		async {
			let mut suggestions = Vec::new();
			if let Some((prefix, completed)) = source.as_slice().split_last() {
				children.suggest(context, completed, *prefix, &mut suggestions).await;
			} else {
				children.suggest(context, &[], "", &mut suggestions).await;
			}
			suggestions
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{
		assert_matches,
		borrow::Cow,
		convert::Infallible,
	};

	use macros::command;

	use crate::{
		StringParser,
		SuggestionProvider,
	};

	struct UserIds;

	#[cfg(not(feature = "async-suggestions"))]
	impl<'a> SuggestionProvider<Vec<&'a str>> for UserIds {
		fn suggest<'c>(&'c self, context: &'c Vec<&'a str>, prefix: &str) -> Vec<Cow<'c, str>>
where {
			context.iter().filter(|id| id.starts_with(prefix)).map(|id| Cow::Borrowed(*id)).collect()
		}
	}

	#[cfg(feature = "async-suggestions")]
	impl<'a> SuggestionProvider<Vec<&'a str>> for UserIds
	where
		Vec<&'a str>: Sync,
	{
		fn suggest<'c>(&'c self, context: &'c Vec<&'a str>, prefix: &str) -> impl Future<Output = Vec<Cow<'c, str>>> + Send {
			async move { context.iter().filter(|id| id.starts_with(prefix)).map(|id| Cow::Borrowed(*id)).collect() }
		}
	}

	macro_rules! command_tree {
		() => {
			command! {
				literal "ban" | "block" executes |_: &mut ()| Ok::<(), Infallible>(())
				literal "user" {
					literal "ban" {
						argument "id": StringParser suggestions UserIds executes |_: &mut (), _: String| Ok::<(), Infallible>(())
					}
				}
				literal "list" executes |_: &mut ()| Ok::<(), Infallible>(())
			}
		};
	}

	#[cfg(not(feature = "async-suggestions"))]
	#[test]
	fn sync_suggestions_match_literals_aliases_and_nested_argument_values() {
		let command = command_tree!();
		let context = vec!["usr-1", "usr-2", "other"];
		assert_matches!(command.suggest(&context, &["b"]), Ok(v) if v == &["ban", "block"]);
		assert_matches!(command.suggest(&context, &["user", "ban", "usr-"]), Ok(v) if v == &["usr-1", "usr-2"]);
		assert_matches!(command.suggest(&context, &["user", "unknown"]), Ok(v) if v.is_empty());
	}

	#[cfg(feature = "async-suggestions")]
	fn run_ready<F: Future>(future: F) -> F::Output {
		use std::task::{
			Context,
			Poll,
			Waker,
		};

		let mut future = std::pin::pin!(future);
		let mut context = Context::from_waker(Waker::noop());
		match future.as_mut().poll(&mut context) {
			Poll::Ready(output) => output,
			Poll::Pending => panic!("test suggestion future unexpectedly yielded"),
		}
	}

	#[cfg(feature = "async-suggestions")]
	#[test]
	fn async_suggestions_match_literals_aliases_and_nested_argument_values() {
		let command = command_tree!();
		// The intent of this test is to pass in a "database connection" as context, which is queried asynchronously to provide suggestions. For simplicity, we just use a vector of strings as the context.
		// The non-async version mimics a cached query, so it can return results immediately from memory.
		// Either version can be used, including just making it all async as the overhead for async is minimal for simple queries even if the underlying source is synchronously queried.
		// However, as the library is designed to be also possible to use in a synchronous context, the non-async version is provided as a separate feature.
		assert_matches!(run_ready(command.suggest(&vec!["usr-1", "usr-2", "other"], &["b"])), Ok(v) if v == &["ban", "block"]);
		assert_matches!(run_ready(command.suggest(&vec!["usr-1", "usr-2", "other"], &["user", "ban", "usr-"])), Ok(v) if v == &["usr-1", "usr-2"]);
		assert_matches!(run_ready(command.suggest(&vec!["usr-1"], &["user", "unknown"])), Ok(v) if v.is_empty());
	}
}
