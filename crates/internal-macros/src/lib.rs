use proc_macro::TokenStream as StdTokenStream;
use proc_macro2::{
	Ident,
	TokenStream,
};
use quote::{
	format_ident,
	quote,
};

#[proc_macro]
pub fn impl_executor(input: StdTokenStream) -> StdTokenStream {
	let int = syn::parse_macro_input!(input as syn::LitInt);
	let count = int.base10_parse::<usize>().expect("Failed to parse integer literal");
	(0..=count).map(executor_impl).collect::<TokenStream>().into()
}

fn executor_impl(arg_count: usize) -> TokenStream {
	let types = (0..arg_count).map(|i| format_ident!("T{}", i)).collect::<Vec<_>>();
	let values = types.iter().map(|ty| format_ident!("{}", ty.to_string().to_lowercase())).collect::<Vec<_>>();
	let stack = recursive_tuple(types.iter());

	if arg_count == 0 {
		return quote! {
			impl<Ctx, Func, Er> Executor<Ctx, ()> for Func
			where
				Func: Fn(&mut Ctx) -> Result<(), Er>,
				Er: std::error::Error + Send + Sync + 'static,
			{
				type Error = Er;

				#[inline(always)]
				fn run(&self, ctx: &mut Ctx, _stack: ()) -> Result<(), Self::Error> {
					self(ctx)
				}
			}
		};
	}

	let pattern = recursive_tuple(values.iter());

	quote! {
		impl<Ctx, Func, #(#types),*, Er> Executor<Ctx, #stack> for Func
		where
			Func: Fn(&mut Ctx, #(#types),*) -> Result<(), Er>,
			Er: std::error::Error + Send + Sync + 'static,
		{
			type Error = Er;

			#[inline(always)]
			fn run(&self, ctx: &mut Ctx, #pattern: #stack) -> Result<(), Self::Error> {
				self(ctx, #(#values),*)
			}
		}
	}
}

fn recursive_tuple<'a>(items: impl Iterator<Item = &'a Ident>) -> TokenStream {
	items.fold(quote! { () }, |tail, item| quote! { (#item, #tail) })
}
