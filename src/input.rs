use std::env::{
	Args,
	ArgsOs,
};

use crate::error::InputError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
	pub offset: usize,
}

/// Command input from a source of tokens.
#[derive(Debug)]
pub struct Input<Source>
where
	Source: InputSource,
{
	source: Source,
	pub offset: usize,
}

impl<Source> Input<Source>
where
	Source: InputSource,
{
	#[inline(always)]
	pub const fn new(source: Source) -> Self {
		Self { source, offset: 0 }
	}

	/// Returns a checkpoint of the current input offset, which can be used to restore the offset later.
	#[inline(always)]
	pub const fn checkpoint(&self) -> Checkpoint {
		Checkpoint { offset: self.offset }
	}

	/// Restores the input offset to a previously saved checkpoint.
	#[inline(always)]
	pub const fn restore(&mut self, checkpoint: Checkpoint) {
		self.offset = checkpoint.offset;
	}

	/// Returns whether there are remaining tokens in the input source from the current offset.
	#[inline(always)]
	pub fn has_remaining(&self) -> bool {
		self.source.has_remaining(self.offset)
	}

	/// Returns the token at the given offset, or `None` if the offset is out of bounds.
	#[inline(always)]
	pub fn get(&self, offset: usize) -> Option<&str> {
		self.source.get(offset)
	}

	/// Returns the next token in the input source, or `None` if there are no more tokens.
	#[inline(always)]
	pub fn peek<'a, 'b>(&'a self) -> Option<&'b str>
	where
		'a: 'b,
	{
		self.source.get(self.offset)
	}

	/// Returns the remaining tokens from the current offset, or `None` if there are no more tokens.
	#[inline(always)]
	pub fn remaining_tokens(&self) -> Option<Vec<&str>> {
		self.remaining_tokens_at(self.offset)
	}

	/// Returns the remaining tokens from the given offset, or `None` if there are no more tokens.
	#[inline(always)]
	pub fn remaining_tokens_at(&self, offset: usize) -> Option<Vec<&str>> {
		// The impl is left to the source, as the source may have more efficient ways to get the remaining rather than just iterating over offset..len and calling get on each
		self.source.remaining_tokens(offset)
	}

	/// Returns the number of tokens in the input source.
	#[inline(always)]
	pub fn len(&self) -> usize {
		self.source.len()
	}

	/// Advances the current offset by the given count.
	#[inline(always)]
	pub fn advance(&mut self, count: usize) {
		self.offset += count;
	}

	/// Advances the offset and returns the next token, or `None` if there are no more tokens.
	#[inline(always)]
	pub fn next_token(&mut self) -> Option<&str> {
		let token = self.source.get(self.offset);
		if token.is_some() {
			self.offset += 1;
		}
		token
	}
}

/// Represents a source of input for the command parser.
/// The input source needs to allow for retrieving tokens at a given offset (random access) and the len of inputs.
/// The input source impl may be the owner, or simply a reference to the underlying data.
/// The input source is expected to be immutable, and the parser will maintain its own offset into the input source.
pub trait InputSource {
	/// Returns the token at the given offset, or `None` if the offset is out of bounds.
	fn get(&self, offset: usize) -> Option<&str>;

	/// Returns the number of tokens in the input source.
	fn len(&self) -> usize;

	/// Returns the tokens at the given offset until the end of the input source, or `None` if the offset is out of bounds.
	#[inline(always)]
	fn remaining_tokens(&self, offset: usize) -> Option<Vec<&str>> {
		if self.has_remaining(offset) {
			Some((offset..self.len()).filter_map(|i| self.get(i)).collect())
		} else {
			None
		}
	}

	/// Whether the input source has no tokens.
	#[inline(always)]
	fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Returns the next token in the input source, or `None` if there are no more tokens.
	#[inline(always)]
	fn has_remaining(&self, offset: usize) -> bool {
		offset < self.len()
	}
}

// region InputSource impls
impl<T: AsRef<str>, const N: usize> InputSource for [T; N] {
	#[inline(always)]
	fn get(&self, offset: usize) -> Option<&str> {
		InputSource::get(self.as_slice(), offset)
	}

	#[inline(always)]
	fn len(&self) -> usize {
		N
	}

	#[inline(always)]
	fn remaining_tokens(&self, offset: usize) -> Option<Vec<&str>> {
		InputSource::remaining_tokens(self.as_slice(), offset)
	}
}

impl<T: AsRef<str>> InputSource for [T] {
	#[inline(always)]
	fn get(&self, offset: usize) -> Option<&str> {
		<[T]>::get(self, offset).map(AsRef::as_ref)
	}

	#[inline(always)]
	fn len(&self) -> usize {
		<[T]>::len(self)
	}

	#[inline(always)]
	fn remaining_tokens(&self, offset: usize) -> Option<Vec<&str>> {
		if offset >= self.len() {
			None
		} else {
			<[T]>::get(self, offset..).map(|s| s.iter().map(AsRef::as_ref).collect())
		}
	}
}

impl<T: AsRef<str>> InputSource for Vec<T> {
	#[inline(always)]
	fn get(&self, offset: usize) -> Option<&str> {
		InputSource::get(self.as_slice(), offset)
	}

	#[inline(always)]
	fn len(&self) -> usize {
		Vec::len(self)
	}

	#[inline(always)]
	fn remaining_tokens(&self, offset: usize) -> Option<Vec<&str>> {
		InputSource::remaining_tokens(self.as_slice(), offset)
	}
}

// Reference impl
impl<T: InputSource + ?Sized> InputSource for &T {
	#[inline(always)]
	fn get(&self, offset: usize) -> Option<&str> {
		(**self).get(offset)
	}

	#[inline(always)]
	fn len(&self) -> usize {
		(**self).len()
	}

	#[inline(always)]
	fn remaining_tokens(&self, offset: usize) -> Option<Vec<&str>> {
		(**self).remaining_tokens(offset)
	}
}
// endregion

/// Convenience trait for types that can be converted into an input source.
///
/// Implementation strategies:
/// - Direct String or &str inputs are split on whitespace and collected into a Vec<String> or Vec<&str> respectively.
/// - Vec and slice inputs with AsRef<str> items are passed through as-is, as they already implement InputSource.
/// - **In the case of command-line arguments, the first argument is skipped as it is the application's name and not considered part of the command input.**
pub trait IntoInputSource {
	type Source: InputSource;

	fn into_input_source(self) -> Result<Self::Source, InputError>;
}

// region  IntoInputSource impls

// Command-line argument impls
impl IntoInputSource for Args {
	type Source = Vec<String>;

	#[inline(always)]
	fn into_input_source(self) -> Result<Vec<String>, InputError> {
		Ok(self.skip(1).collect())
	}
}
impl IntoInputSource for ArgsOs {
	type Source = Vec<String>;

	#[inline(always)]
	fn into_input_source(self) -> Result<Self::Source, InputError> {
		self.skip(1).map(|s| s.into_string().map_err(InputError::InvalidUtf8)).collect()
	}
}
#[cfg(feature = "argv")]
impl IntoInputSource for argv::Iter {
	type Source = Vec<&'static str>;

	#[inline(always)]
	fn into_input_source(self) -> Result<Self::Source, InputError> {
		self.skip(1).map(|s| s.to_str().ok_or_else(|| InputError::InvalidUtf8(s.to_owned()))).collect()
	}
}

// Whitespace-splitting impls
impl IntoInputSource for String {
	type Source = Vec<String>;

	#[inline(always)]
	fn into_input_source(self) -> Result<Vec<String>, InputError> {
		Ok(self.split_whitespace().map(|s| s.to_string()).collect())
	}
}
impl<'a> IntoInputSource for &'a String {
	type Source = Vec<&'a str>;

	#[inline(always)]
	fn into_input_source(self) -> Result<Self::Source, InputError> {
		Ok(self.split_whitespace().collect())
	}
}
impl<'a> IntoInputSource for &'a str {
	type Source = Vec<&'a str>;

	#[inline(always)]
	fn into_input_source(self) -> Result<Self::Source, InputError> {
		Ok(self.split_whitespace().collect())
	}
}

// Reflexive impl for any type that implements InputSource, so that it can be used directly as an input source.
impl<T: InputSource> IntoInputSource for T {
	type Source = T;

	#[inline(always)]
	fn into_input_source(self) -> Result<Self::Source, InputError> {
		Ok(self)
	}
}

// endregion
