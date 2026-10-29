#[cfg(feature = "ansi")]
use std::fmt::Write as _;

use crate::{
	Input,
	InputSource,
	NodeMetadata,
	WriteNode,
};

#[derive(Clone, Copy)]
enum HelpStyle {
	Heading,
	Literal,
	Placeholder,
	ArgumentType,
}

#[inline(always)]
fn push_styled(output: &mut String, text: &str, style: HelpStyle) {
	#[cfg(feature = "ansi")]
	{
		let code = match style {
			HelpStyle::Heading => "1;32",
			HelpStyle::Literal => "1;34",
			HelpStyle::Placeholder => "36",
			HelpStyle::ArgumentType => "1;33",
		};
		let _ = write!(output, "\x1b[{code}m{text}\x1b[0m");
	}
	#[cfg(not(feature = "ansi"))]
	{
		let _ = style;
		output.push_str(text);
	}
}

fn push_argument_placeholder(output: &mut String, placeholder: &str) {
	if let Some((name, value_type)) = placeholder.strip_prefix('<').and_then(|inner| inner.strip_suffix('>')).and_then(|inner| inner.split_once(": ")) {
		let prefix_len = name.len() + 3;
		push_styled(output, &placeholder[..prefix_len], HelpStyle::Placeholder);
		push_styled(output, value_type, HelpStyle::ArgumentType);
		push_styled(output, ">", HelpStyle::Placeholder);
	} else {
		push_styled(output, placeholder, HelpStyle::Placeholder);
	}
}

fn push_styled_usage(output: &mut String, usage: &str) {
	let mut remaining = usage;
	while let Some(start) = remaining.find('<') {
		push_styled(output, &remaining[..start], HelpStyle::Heading);
		let placeholder = &remaining[start..];
		let Some(end) = placeholder.find('>') else {
			push_styled(output, placeholder, HelpStyle::Heading);
			return;
		};
		push_argument_placeholder(output, &placeholder[..=end]);
		remaining = &placeholder[end + 1..];
	}
	push_styled(output, remaining, HelpStyle::Heading);
}

/// Formats inspected command metadata as a concise help page.
///
/// The `path` is the command path selected after the `help` command. With the
/// `ansi` feature enabled, the returned string contains ANSI escape codes.
pub(crate) fn format_help_output(tree_output: &str, path: &[&str]) -> String {
	let mut output = String::with_capacity(tree_output.len() + 64);
	push_styled(&mut output, "Usage:", HelpStyle::Heading);
	output.push('\n');
	output.push_str("  ");
	let usage = format_usage(tree_output, path);
	push_styled_usage(&mut output, &usage);
	output.push_str("\n\n");

	if !tree_output.is_empty() {
		push_styled(&mut output, "Commands:", HelpStyle::Heading);
		output.push('\n');
		for line in tree_output.lines() {
			let indent = &line[..line.len() - line.trim_start().len()];
			let content = line.trim_start();
			output.push_str(indent);
			if content.starts_with('<') {
				if let Some(end) = content.find(">: ") {
					push_argument_placeholder(&mut output, &content[..=end]);
					output.push_str(&content[end + 1..]);
				} else {
					output.push_str(content);
				}
			} else if let Some((literal, description)) = content.split_once(": ") {
				push_styled(&mut output, literal, HelpStyle::Literal);
				output.push_str(": ");
				output.push_str(description);
			} else {
				output.push_str(content);
			}
			output.push('\n');
		}
		output.push('\n');
	}

	output
}

fn format_usage(tree_output: &str, path: &[&str]) -> String {
	if path.is_empty() {
		return "<command> [<arguments>...]".to_owned();
	}

	let known_literals: Vec<Vec<&str>> = tree_output
		.lines()
		.filter_map(|line| line.trim_start().split_once(": ").map(|(name, _)| name))
		.filter(|name| !name.starts_with('<'))
		.map(|name| name.split(" | ").collect())
		.collect();
	let command_path: Vec<&str> = path.iter().copied().filter(|token| known_literals.iter().any(|aliases| aliases.contains(token))).collect();
	if command_path.is_empty() {
		return "<command> [<arguments>...]".to_owned();
	}

	let mut ancestors: Vec<(usize, Vec<&str>)> = Vec::new();
	let mut target = None;
	for (index, line) in tree_output.lines().enumerate() {
		let indent = line.len() - line.trim_start().len();
		let content = line.trim_start();
		while ancestors.last().is_some_and(|(parent_indent, _)| *parent_indent >= indent) {
			ancestors.pop();
		}
		let Some((name, _)) = content.split_once(": ") else {
			continue;
		};
		if name.starts_with('<') {
			continue;
		}

		let aliases: Vec<&str> = name.split(" | ").collect();
		let matches_path = ancestors.len() + 1 == command_path.len()
			&& ancestors.iter().zip(&command_path).all(|((_, parent_aliases), token)| parent_aliases.contains(token))
			&& aliases.contains(command_path.last().unwrap());
		if matches_path {
			target = Some((index, indent));
		}
		ancestors.push((indent, aliases));
	}

	let mut usage = command_path.join(" ");
	if let Some((target_index, target_indent)) = target {
		let mut argument_indent = target_indent;
		let mut has_child_command = false;
		for line in tree_output.lines().skip(target_index + 1) {
			let indent = line.len() - line.trim_start().len();
			let content = line.trim_start();
			if indent <= target_indent {
				break;
			}
			if content.starts_with('<') {
				if indent <= argument_indent {
					break;
				}
				if let Some(end) = content.find(">: ") {
					usage.push(' ');
					usage.push_str(&content[..=end]);
					argument_indent = indent;
				}
			} else {
				has_child_command = true;
				break;
			}
		}
		if has_child_command && usage == command_path.join(" ") {
			usage.push_str(" <command>");
		}
	}

	usage
}

/// A visitor that collects metadata about nodes. This is used to generate the /help command.
pub struct NodeInspector {
	pub output: String,
	depth: usize,
}

impl NodeInspector {
	#[inline(always)]
	pub fn new() -> Self {
		Self {
			output: String::with_capacity(256),
			depth: 0,
		}
	}

	#[inline(always)]
	pub fn reset(&mut self) {
		self.output.clear();
		self.depth = 0;
	}

	#[inline(always)]
	pub fn inspect<Node>(&mut self) -> Result<(), InspectionError>
	where
		Node: WriteNode,
	{
		let indent = "  ".repeat(self.depth + 1);
		self.output.push_str(&indent);
		Node::write_node(&mut self.output).map_err(|_| InspectionError::IncompleteCommand)?;
		self.output.push('\n');

		Ok(())
	}

	#[inline(always)]
	pub(crate) fn branch(&self) -> Self {
		Self {
			output: String::with_capacity(256),
			depth: self.depth,
		}
	}

	#[inline(always)]
	pub(crate) fn at_child<F>(&mut self, inspect: F) -> Result<(), InspectionError>
	where
		F: FnOnce(&mut Self) -> Result<(), InspectionError>,
	{
		self.depth += 1;
		let result = inspect(self);
		self.depth -= 1;
		result
	}

	#[inline(always)]
	pub(crate) fn append(&mut self, other: Self) {
		self.output.push_str(&other.output);
	}
}

/// Nodes that can be inspected. This is used to generate the /help command.
pub trait InspectableNode<Source, Inspectability = <Self as NodeMetadata>::Inspect>
where
	Source: InputSource,
	Inspectability: InspectCapability,
	Self: NodeMetadata,
{
	fn inspect(&self, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError>;
}

#[derive(Debug, thiserror::Error)]
pub enum InspectionError {
	#[error("Unknown command at input offset {offset}")]
	UnknownCommand { offset: usize },
	#[error("Incomplete command")]
	IncompleteCommand,
	#[error(transparent)]
	Input(#[from] crate::InputError),
}

impl InspectionError {
	pub(crate) fn into_command_error<Source>(self, input: &Input<Source>) -> crate::CommandError
	where
		Source: InputSource,
	{
		match self {
			InspectionError::UnknownCommand { offset } => input
				.get(offset)
				.map(|token| crate::CommandError::UnknownCommand(token.to_owned()))
				.unwrap_or(crate::CommandError::IncompleteCommand),
			InspectionError::IncompleteCommand => crate::CommandError::IncompleteCommand,
			InspectionError::Input(error) => crate::CommandError::InputError(error),
		}
	}
}

/// Sum type for the inspection capabilities of a node
pub trait CombineInspect<Rhs> {
	type Output: InspectCapability;
}

impl CombineInspect<NoneInspect> for NoneInspect {
	type Output = NoneInspect;
}
impl CombineInspect<HeadInspect> for NoneInspect {
	type Output = HeadInspect;
}
impl CombineInspect<TailInspect> for NoneInspect {
	type Output = TailInspect;
}
impl CombineInspect<NoneInspect> for HeadInspect {
	type Output = HeadInspect;
}
impl CombineInspect<NoneInspect> for TailInspect {
	type Output = TailInspect;
}
impl CombineInspect<HeadInspect> for TailInspect {
	type Output = BothInspect;
}
impl CombineInspect<TailInspect> for HeadInspect {
	type Output = BothInspect;
}
impl CombineInspect<HeadInspect> for HeadInspect {
	type Output = BothInspect;
}
impl CombineInspect<TailInspect> for TailInspect {
	type Output = BothInspect;
}
impl CombineInspect<BothInspect> for NoneInspect {
	type Output = BothInspect;
}
impl CombineInspect<NoneInspect> for BothInspect {
	type Output = BothInspect;
}
impl CombineInspect<BothInspect> for HeadInspect {
	type Output = BothInspect;
}
impl CombineInspect<HeadInspect> for BothInspect {
	type Output = BothInspect;
}
impl CombineInspect<BothInspect> for TailInspect {
	type Output = BothInspect;
}
impl CombineInspect<TailInspect> for BothInspect {
	type Output = BothInspect;
}
impl CombineInspect<BothInspect> for BothInspect {
	type Output = BothInspect;
}

/// This node has no metadata suggestions of its own. Parent nodes can use this capability to skip inspecting it.
pub struct NoneInspect;
/// Only the head contributes inspection suggestions when capabilities are combined.
pub struct HeadInspect;
/// Only the tail contributes inspection suggestions when capabilities are combined.
pub struct TailInspect;
/// Both head and tail contribute inspection suggestions when capabilities are combined.
pub struct BothInspect;

pub trait InspectCapability {}
impl InspectCapability for NoneInspect {}
impl InspectCapability for HeadInspect {}
impl InspectCapability for TailInspect {}
impl InspectCapability for BothInspect {}

/// Dispatches inspection based on the child's compile-time capability.
/// `NoneInspect` skips the child completely; the other capabilities inspect it and
/// fall back to the parent metadata when the child has no more specific result.
#[doc(hidden)]
pub trait InspectChild<Child, Source, Parent>
where
	Source: InputSource,
	Parent: NodeMetadata,
{
	fn inspect_child(child: &Child, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError>;
}

/// Handles child traversal when no token is available yet.
#[doc(hidden)]
pub trait InspectMissingChild<Child, Source>
where
	Source: InputSource,
{
	fn inspect_missing(child: &Child, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError>;
}

impl<Child, Source, Parent> InspectChild<Child, Source, Parent> for NoneInspect
where
	Source: InputSource,
	Parent: NodeMetadata,
{
	fn inspect_child(_child: &Child, _inspector: &mut NodeInspector, _input: &mut Input<Source>) -> Result<(), InspectionError> {
		Ok(())
	}
}

macro_rules! impl_inspect_child {
	($capability:ty) => {
		impl<Child, Source, Parent> InspectChild<Child, Source, Parent> for $capability
		where
			Source: InputSource,
			Parent: NodeMetadata + WriteNode,
			Child: InspectableNode<Source>,
		{
			fn inspect_child(child: &Child, inspector: &mut NodeInspector, input: &mut Input<Source>) -> Result<(), InspectionError> {
				let _ = inspector.at_child(|inspector| child.inspect(inspector, input));
				Ok(())
			}
		}
	};
}

impl_inspect_child!(HeadInspect);
impl_inspect_child!(TailInspect);
impl_inspect_child!(BothInspect);
