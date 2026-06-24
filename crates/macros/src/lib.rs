use proc_macro2::TokenStream;
use proc_macro_crate::crate_name;
use quote::{
	format_ident,
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
use syn::token::Or;

mod kw {
	use syn::custom_keyword;

	custom_keyword!(literal);
	custom_keyword!(argument);
	custom_keyword!(executes);
}

const MAIN_CRATE_NAME: &str = "command";

fn crate_path() -> TokenStream {
	match crate_name(MAIN_CRATE_NAME).expect("Could not find crate") {
		proc_macro_crate::FoundCrate::Itself => quote!(crate),
		proc_macro_crate::FoundCrate::Name(name) => {
			let ident = format_ident!("{name}");
			quote!(::#ident)
		}
	}
}

#[proc_macro]
pub fn command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let tree = parse_macro_input!(input as CommandTree);
	let crate_path = crate_path();
	quote! {
		#crate_path::Root::new(#tree)
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
			let mut seen_names = std::collections::HashSet::new();
			for node in &self.root_nodes {
				if let CommandNode::Literal { names, .. } = node {
					for name in names {
						if !seen_names.insert(name.value()) {
							return Err(syn::Error::new_spanned(name, "Duplicate literal name"));
						}
					}
				}
			}
		} else if self.root_nodes.iter().all(|node| matches!(node, CommandNode::Argument { .. })) {
			let mut seen_names = std::collections::HashSet::new();
			for node in &self.root_nodes {
				if let CommandNode::Argument { name, .. } = node {
					if !seen_names.insert(name.value()) {
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
		let crate_path = crate_path();
		if len == 0 {
			tokens.extend(quote!(#crate_path::Nil));
		} else if len == 1 {
			let node = &root_nodes[0];
			tokens.extend(node.to_token_stream());
		} else {
			let mut nodes: Vec<_> = root_nodes.iter().map(ToTokens::to_token_stream).collect();

			let mut acc = nodes.pop().unwrap(); // Safe since we're above len == 1
			while let Some(node) = nodes.pop() {
				acc = quote! {
					#crate_path::Choice::new(#node, #acc)
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
		names: Vec<LitStr>,
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
			let mut names: Vec<LitStr> = Vec::new();
			names.push(input.parse()?); // Enforces at least one name

			while input.peek(Or) && input.peek2(LitStr) {
				let _or: Or = input.parse()?;
				names.push(input.parse()?);
			}

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

			Ok(CommandNode::Literal { names, children, executor })
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
		let crate_path = crate_path();
		let node_tokens = match self {
			CommandNode::Literal { names, children, executor } => {
				let ty = match names.len() {
					0 => unreachable!("Literal node must have at least one name"),
					1 => {
						let name = &names[0];
						quote! { #crate_path::Literal::<#name, _> }
					}
					_ => quote! { #crate_path::LiteralAliases::<{ &[#(#names),*] }, _>},
				};
				match executor {
					Some(executor) => match children.root_nodes.len() {
						0 => quote! { #ty::new(#executor) },
						_ => quote! { #ty::new(#crate_path::WithExec::new(#children, #executor)) },
					},
					// `children` is guaranteed to be more than 0 if `executor` is None via parse-time validation, so this is safe
					None => quote! { #ty::new(#children) },
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
						0 => quote! { #crate_path::Argument::<#name, #parser_type, _>::new(<#parser_type as Default>::default(), #executor) },
						_ => quote! { #crate_path::Argument::<#name, #parser_type, _>::new(<#parser_type as Default>::default(), #crate_path::WithExec::new(#children, #executor)) },
					},
					// `children` is guaranteed to be more than 0 if `executor` is None via parse-time validation, so this is safe
					None => quote! { #crate_path::Argument::<#name, #parser_type, _>::new(<#parser_type as Default>::default(), #children) },
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
		if input.peek(syn::Ident) || input.peek(Or) || input.peek(syn::token::Move) || input.peek(syn::token::Async) {
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
		let crate_path = crate_path();
		tokens.extend(quote! {
			#crate_path::Exec::new(#expr)
		});
	}
}
// endregion
