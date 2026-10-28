mod root;

use std::fmt::Write;

pub use root::*;
mod literal;
pub use literal::*;
mod literal_aliases;
pub use literal_aliases::*;
mod argument;
pub use argument::*;
mod exec;
pub use exec::*;
mod with_exec;
pub use with_exec::*;
mod nil;
pub use nil::*;
mod choice;
pub use choice::*;

use crate::{
	Input,
	InputSource,
	InspectMissingChild,
	InspectionError,
	NodeInspector,
};

/// A trait that represents the kind of a node. This is used to determine how to inspect the node and its children.
pub trait NodeKind {}

/// The default node kind for nodes that do not have any children to inspect. Any node that does not have any children to inspect should use this kind.
pub struct NoInspectKind;

impl NodeKind for NoInspectKind {}

/// A default implementation of `InspectMissingChild` that does nothing. This is used for nodes that do not have any children to inspect.
impl<Child, Source> InspectMissingChild<Child, Source> for NoInspectKind
where
	Source: InputSource,
{
	#[inline(always)]
	fn inspect_missing(_child: &Child, _inspector: &mut NodeInspector, _input: &mut Input<Source>) -> Result<(), InspectionError> {
		Ok(())
	}
}

/// A trait for nodes that can be written to a buffer.
///
/// This is used as an alternative to the stdlib's Display, which needs &self to write when we can operate directly on the type itself.
pub trait WriteNode {
	fn write_node<Buf>(buf: &mut Buf) -> core::fmt::Result
	where
		Buf: Write;
}
