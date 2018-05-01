#![feature(proc_macro)]

extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use std::iter;

#[proc_macro]
pub fn wstr(input: TokenStream) -> TokenStream {
	let ast: syn::LitStr = syn::parse(input).expect("not a string literal");
	let utf8_str = ast.value();

	let wchars = utf8_str.encode_utf16().chain(iter::once('\0' as u16));

	let expanded = quote! {
		&[#(#wchars),*] as LPCWSTR
	};

	expanded.into()
}
