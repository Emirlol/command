use proc_macro2::TokenStream;
use quote::{
	quote,
	ToTokens,
};
use syn::{
	braced,
	parse::{
		Parse,
		ParseStream,
	},
	parse_macro_input,
	token::Colon,
	LitStr,
};

mod kw {
	use syn::custom_keyword;

	custom_keyword!(literal);
	custom_keyword!(argument);
	custom_keyword!(executes);
}

#[proc_macro]
pub fn command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let tree = parse_macro_input!(input as CommandTree);
	quote! {
		Root::new(#tree)
	}
	.into_token_stream()
	.into()
}

// region CommandTree
struct CommandTree {
	root_nodes: Vec<CommandNode>,
}

impl CommandTree {
	fn validate_nodes(&self, input: &ParseStream) -> syn::Result<()> {
		if self.root_nodes.iter().all(|node| matches!(node, CommandNode::Literal { .. })) {
			let mut names = std::collections::HashSet::new();
			for node in &self.root_nodes {
				if let CommandNode::Literal { name, .. } = node {
					if !names.insert(name.value()) {
						return Err(syn::Error::new_spanned(name, "Duplicate literal name"));
					}
				}
			}
		} else if self.root_nodes.iter().all(|node| matches!(node, CommandNode::Argument { .. })) {
			let mut names = std::collections::HashSet::new();
			for node in &self.root_nodes {
				if let CommandNode::Argument { name, .. } = node {
					if !names.insert(name.value()) {
						return Err(syn::Error::new_spanned(name, "Duplicate argument name"));
					}
				}
			}
		} else {
			return Err(input.error("All root nodes must be of the same type (either all literals or all arguments)"));
		}

		Ok(())
	}
}

impl Parse for CommandTree {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut nodes = Vec::new();

		loop {
			if input.peek(kw::literal) || input.peek(kw::argument) {
				nodes.push(input.parse()?);
			} else {
				break;
			}
		}

		let tree = CommandTree { root_nodes: nodes };
		tree.validate_nodes(&input)?;
		Ok(tree)
	}
}

impl ToTokens for CommandTree {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let CommandTree { root_nodes } = self;
		let len = root_nodes.len();
		if len == 0 {
			tokens.extend(quote!(Nil));
		} else if len == 1 {
			let node = &root_nodes[0];
			tokens.extend(node.to_token_stream());
		} else {
			let mut nodes: Vec<_> = root_nodes.iter().map(ToTokens::to_token_stream).collect();

			let mut acc = nodes.pop().unwrap(); // Safe since we're above len == 1
			while let Some(node) = nodes.pop() {
				acc = quote! {
					Choice::new(#node, #acc)
				};
			}

			tokens.extend(acc);
		}
	}
}
// endregion

// region CommandNode
enum CommandNode {
	Literal {
		name: LitStr,
		children: CommandTree,
		executor: Option<Executor>,
	},
	Argument {
		name: LitStr,
		parser_type: syn::Type,
		children: CommandTree,
		executor: Option<Executor>,
	},
}

impl Parse for CommandNode {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		if input.peek(kw::literal) {
			let _literal: kw::literal = input.parse()?;
			let name: LitStr = input.parse()?;
			let mut executor: Option<Executor> = None;
			let mut children: Vec<CommandNode> = Vec::new();
			if input.peek(kw::executes) {
				executor = Some(input.parse()?);
			} else {
				let content;
				braced!(content in input);

				loop {
					if content.peek(kw::literal) || content.peek(kw::argument) {
						children.push(content.parse()?);
					} else if content.peek(kw::executes) {
						if executor.is_some() {
							return Err(content.error("Multiple `executes` blocks are not allowed"));
						}
						executor = Some(content.parse()?);
					} else {
						break;
					}
				}
			}

			if children.is_empty() && executor.is_none() {
				return Err(input.error("Literal node must have either an executor via `executes` or child nodes"));
			}

			let children = CommandTree { root_nodes: children };
			children.validate_nodes(&input)?;

			Ok(CommandNode::Literal { name, children, executor })
		} else if input.peek(kw::argument) {
			let _argument: kw::argument = input.parse()?;
			let name: LitStr = input.parse()?;
			let _colon: Colon = input.parse()?;
			let parser_type: syn::Type = input.parse()?;
			let mut executor: Option<Executor> = None;
			let mut children: Vec<CommandNode> = Vec::new();
			if input.peek(kw::executes) {
				executor = Some(input.parse()?);
			} else {
				let content;
				braced!(content in input);

				loop {
					if content.peek(kw::literal) || content.peek(kw::argument) {
						children.push(content.parse()?);
					} else if content.peek(kw::executes) {
						if executor.is_some() {
							return Err(content.error("Multiple `executes` blocks are not allowed"));
						}
						executor = Some(content.parse()?);
					} else {
						break;
					}
				}
			}

			if children.is_empty() && executor.is_none() {
				return Err(input.error("Argument node must have either an executor via `executes` or child nodes"));
			}

			let children = CommandTree { root_nodes: children };
			children.validate_nodes(&input)?;

			Ok(CommandNode::Argument {
				name,
				parser_type,
				children,
				executor,
			})
		} else {
			Err(input.error("Expected `literal` or `argument`"))
		}
	}
}

impl ToTokens for CommandNode {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let node_tokens = match self {
			CommandNode::Literal { name, children, executor } => {
				match executor {
					Some(executor) => match children.root_nodes.len() {
						0 => quote! { Literal::<#name, _>::new(#executor) },
						_ => quote! { Literal::<#name, _>::new(WithExec::new(#children, #executor)) },
					},
					// `children` is guaranteed to be more than 0 if `executor` is None via parse-time validation, so this is safe
					None => quote! { Literal::<#name, _>::new(#children) },
				}
			}
			CommandNode::Argument {
				name,
				parser_type,
				children,
				executor,
			} => {
				match executor {
					Some(executor) => match children.root_nodes.len() {
						0 => quote! { Argument::<#name, #parser_type, _>::new(<#parser_type as Default>::default(), #executor) },
						_ => quote! { Argument::<#name, #parser_type, _>::new(<#parser_type as Default>::default(), WithExec::new(#children, #executor)) },
					},
					// `children` is guaranteed to be more than 0 if `executor` is None via parse-time validation, so this is safe
					None => quote! { Argument::<#name, #parser_type, _>::new(<#parser_type as Default>::default(), #children) },
				}
			}
		};

		tokens.extend(node_tokens);
	}
}
// endregion

// region Executor
struct Executor {
	expr: syn::Expr,
}

impl Parse for Executor {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let _executes: kw::executes = input.parse()?;
		// Function name or closure start
		if input.peek(syn::Ident) || input.peek(syn::token::Or) || input.peek(syn::token::Move) || input.peek(syn::token::Async) {
			let expr = input.parse()?;
			Ok(Executor { expr })
		} else {
			Err(input.error("Expected function name or closure"))
		}
	}
}

impl ToTokens for Executor {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let Executor { expr } = self;
		tokens.extend(quote! {
			Exec::new(#expr)
		});
	}
}
// endregion
