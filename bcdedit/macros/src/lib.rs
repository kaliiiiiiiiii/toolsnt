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

	let Context {
		str_matchers,
		id_matchers,
		enum_of_enums,
		enum_of_enums_matchers,
		..
	} = ctx;

	quote!(mod generated {
		#![allow(missing_docs)]
		pub(super) fn __element_from_str(string: &::std::primitive::str) -> ::std::option::Option<crate::elements::DynamicElement> {
			match string {
				#str_matchers
				_ => ::core::option::Option::None,
			}
		}

		pub(super) const fn __element_to_str(id: ::std::primitive::u32) -> ::std::option::Option<&'static ::std::primitive::str> {
			match id {
				#id_matchers
				_ => ::core::option::Option::None,
			}
		}

		/// Enum of all support BCD enums
		#[derive(Clone, Copy, Debug, PartialEq, Eq)]
		pub enum Enum {
			#enum_of_enums
		}

		impl Enum {
			pub(super) fn __for_element_value(id: ::std::primitive::u32, value: ::std::primitive::u64) -> Option<Self> {
				match id {
					#enum_of_enums_matchers
					_ => None,
				}
			}
		}

		#categories
	}).into()
}

#[derive(Default)]
struct Context {
	str_matchers: TokenStream,
	id_matchers: TokenStream,
	enum_of_enums: TokenStream,
	enum_of_enums_matchers: TokenStream,
	submodule: Vec<Ident>,
}

fn map_category(ctx: &mut Context, category: Category) -> TokenStream {
	let name = ident(&category.name);
	ctx.submodule.push(name.clone());

	let subcategories = category
		.subcategories
		.into_iter()
		.map(|cat| map_category(ctx, cat))
		.collect::<TokenStream>();

	let enums = category
		.enums
		.into_iter()
		.map(|enum_| map_enum(&mut ctx.enum_of_enums, &ctx.submodule, enum_));

	let elements = category.elements.into_iter().map(|e| {
		map_element(
			&mut ctx.str_matchers,
			&mut ctx.id_matchers,
			&mut ctx.enum_of_enums_matchers,
			e,
		)
	});

	let tt = quote! {
		pub mod #name {
			#(#enums)*
			#(#elements)*
			#subcategories
		}
	};

	ctx.submodule.pop();
	tt
}

fn map_enum(enum_of_enums: &mut TokenStream, submodule: &[Ident], enum_: Enum) -> TokenStream {
	let name = ident(&enum_.name);
	let raw_variants = enum_.variants.iter().map(|EnumVariant { name, value }| {
		let name = (!name.starts_with(char::is_numeric))
			.then(|| ident(&name))
			.unwrap_or_else(|| format_ident!("_{name}"));

		let value = Literal::u64_unsuffixed(*value);
		(name, value)
	});

	let variants = raw_variants
		.clone()
		.map(|(name, value)| quote!(#name = #value));

	let try_dwords =
		raw_variants.map(|(name, value)| quote!(#value => ::std::result::Result::Ok(Self::#name),));

	enum_of_enums.extend(quote! {
		#name(#(#submodule)::*::#name),
	});

	quote! {
		#[derive(Clone, Copy, Debug, PartialEq, Eq)]
		pub enum #name {
			#(#variants,)*
		}

		impl ::std::convert::TryFrom<::std::primitive::u64> for #name {
			type Error = ();

			fn try_from(value: ::std::primitive::u64) -> ::std::result::Result<Self, Self::Error> {
				match value {
					#(#try_dwords)*
					_ => Err(()),
				}
			}
		}

		impl ::std::convert::From<#name> for ::std::primitive::u64 {
			fn from(value: #name) -> Self {
				value as ::std::primitive::u64
			}
		}
	}
}

fn map_element(
	str_matchers: &mut TokenStream,
	id_matchers: &mut TokenStream,
	enum_of_enums_matchers: &mut TokenStream,
	element: Element,
) -> TokenStream {
	// todo: Unknown elements
	let Some(name) = element.name else {
		return TokenStream::new();
	};

	let name_ident = ident(&name.TO_SHOUTY_SNEK_CASE());
	let element_id = Literal::u32_unsuffixed(element.id);

	id_matchers.extend(quote! {
		#element_id => ::core::option::Option::Some(#name),
	});

	if let Some(enum_) = element.r#enum {
		let enum_ = if !enum_.is_empty() {
			ident(&enum_)
		} else {
			ident(&name)
		};

		enum_of_enums_matchers.extend(quote! {
			#element_id =>
				::std::convert::TryFrom::try_from(value)
					.ok()
					.map(Self::#enum_),
		});
	}

	let matchers = str_matcher_patterns(name);
	str_matchers.extend(quote! {
		#(#matchers)|* =>
			::core::option::Option::Some(crate::elements::DynamicElement(#element_id)),
	});

	quote! {
		pub const #name_ident: crate::elements::DynamicElement =
			crate::elements::DynamicElement(#element_id);
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
