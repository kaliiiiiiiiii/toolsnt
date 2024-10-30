mod schema;

use {
	heck::ToShoutySnekCase,
	proc_macro::TokenStream as TokenStream1,
	proc_macro2::{Ident, Literal, Span, TokenStream},
	quote::{format_ident, quote},
	schema::{Category, Element, Enum, EnumVariant},
};

/// Define BCD elements and values based on KDL spcification
#[proc_macro]
pub fn define_elements(ts: TokenStream1) -> TokenStream1 {
	assert!(ts.is_empty(), "This macro doesn't take any parameters");

	schema::decode(include_str!("../../elements/elements.kdl"))
		.into_iter()
		.map(map_category)
		.collect::<TokenStream>()
		.into()
}

fn map_category(category: Category) -> TokenStream {
	let name = ident(&category.name);
	let enums = category.enums.into_iter().map(map_enum);
	let elements = category.elements.into_iter().map(map_element);
	let subcategories = category.subcategories.into_iter().map(map_category);

	quote! {
		pub mod #name {
			#(#enums)*
			#(#elements)*
			#(#subcategories)*
		}
	}
}

fn map_enum(enum_: Enum) -> TokenStream {
	let name = ident(&enum_.name);
	let variants = enum_
		.variants
		.into_iter()
		.map(|EnumVariant { name, value }| {
			let name = (!name.starts_with(char::is_numeric))
				.then(|| ident(&name))
				.unwrap_or_else(|| format_ident!("_{name}"));

			let value = Literal::u64_unsuffixed(value);
			quote!(#name = #value)
		});

	quote! {
		#[derive(Clone, Copy, Debug, PartialEq, Eq)]
		pub enum #name {
			#(#variants,)*
		}
	}
}

fn map_element(element: Element) -> TokenStream {
	// todo: Unknown elements
	let Some(name) = element.name else {
		return TokenStream::new();
	};

	let name = ident(&name.TO_SHOUTY_SNEK_CASE());
	let id = Literal::u32_unsuffixed(element.id);

	quote! {
		pub const #name: crate::elements::DynamicElement =
			crate::elements::DynamicElement(#id);
	}
}

fn ident(string: &str) -> Ident {
	Ident::new(string, Span::mixed_site())
}
