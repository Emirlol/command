use proc_macro2::{
	Ident,
	TokenStream,
};
use quote::{
	format_ident,
	quote,
};

#[proc_macro]
pub fn impl_executor(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
	(0..=26).map(executor_impl).collect::<TokenStream>().into()
}

fn executor_impl(arg_count: usize) -> TokenStream {
	let types = (b'A'..=b'Z').take(arg_count).map(|letter| format_ident!("{}", letter as char)).collect::<Vec<_>>();
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
			fn run(&self, ctx: &mut Ctx, #pattern: #stack) -> Result<(), Self::Error> {
				self(ctx, #(#values),*)
			}
		}
	}
}

fn recursive_tuple<'a>(items: impl Iterator<Item = &'a Ident>) -> TokenStream {
	items.fold(quote! { () }, |tail, item| quote! { (#item, #tail) })
}
