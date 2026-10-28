use crate::{
	InspectCapability,
	NodeKind,
};

pub trait WithMetadata {
	type Metadata: NodeMetadata;
}

pub trait NodeMetadata {
	type Inspect: InspectCapability;
	type Kind: NodeKind;
}
