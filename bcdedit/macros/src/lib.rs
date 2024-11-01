mod schema;

use {
	heck::{ToKebabCase, ToShoutySnekCase},
	proc_macro::TokenStream as TokenStream1,
	proc_macro2::{Ident, Literal, Span, TokenStream},
	quote::{format_ident, quote},
	schema::{Category, Element, Enum, EnumVariant},
};

/// Define BCD elements and values based on KDL spcification
#[proc_macro]
pub fn define_elements(ts: TokenStream1) -> TokenStream1 {
	assert!(ts.is_empty(), "This macro doesn't take any parameters");

	let mut ctx = Context::default();
	let categories = schema::decode(include_str!("../../elements/elements.kdl"))
		.into_iter()
		.map(|category| map_category(&mut ctx, category))
		.collect::<TokenStream>();

	let match_inner = ctx.str_matchers;
	quote! {
		fn __element_from_str(string: &::std::primitive::str) -> ::std::option::Option<crate::elements::DynamicElement> {
			match string {
				#match_inner
				_ => None,
			}
		}

		#categories
	}.into()
}

#[derive(Default)]
struct Context {
	str_matchers: TokenStream,
}

fn map_category(ctx: &mut Context, category: Category) -> TokenStream {
	let subcategories = category
		.subcategories
		.into_iter()
		.map(|cat| map_category(ctx, cat))
		.collect::<TokenStream>();

	let name = ident(&category.name);
	let enums = category.enums.into_iter().map(map_enum);
	let elements = category
		.elements
		.into_iter()
		.map(|e| map_element(&mut ctx.str_matchers, e));

	quote! {
		pub mod #name {
			#(#enums)*
			#(#elements)*
			#subcategories
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

fn map_element(str_matchers: &mut TokenStream, element: Element) -> TokenStream {
	// todo: Unknown elements
	let Some(name) = element.name else {
		return TokenStream::new();
	};

	let ident = ident(&name.TO_SHOUTY_SNEK_CASE());
	let id = Literal::u32_unsuffixed(element.id);

	let matchers = str_matcher_patterns(name);
	str_matchers.extend(quote! {
		#(#matchers)|* => Some(crate::elements::DynamicElement(#id)),
	});

	quote! {
		pub const #ident: crate::elements::DynamicElement =
			crate::elements::DynamicElement(#id);
	}
}

fn str_matcher_patterns(name: String) -> impl Iterator<Item = Literal> {
	let kebab = name.to_kebab_case();
	let snek = name.TO_SHOUTY_SNEK_CASE();

	[
		Literal::string(&name),
		Literal::string(&kebab),
		Literal::string(&snek),
	]
	.into_iter()
}

fn ident(string: &str) -> Ident {
	Ident::new(string, Span::mixed_site())
}
