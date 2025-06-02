mod schema;

use {
	heck::{ToKebabCase, ToShoutySnekCase},
	proc_macro::TokenStream as TokenStream1,
	proc_macro2::{Ident, Literal, Span, TokenStream},
	quote2::{format_ident, quote, Quote},
	schema::{Category, Element, Enum, EnumVariant, Requires},
};

/// Define BCD elements and values based on KDL specification
#[proc_macro]
pub fn define_elements(ts: TokenStream1) -> TokenStream1 {
	assert!(ts.is_empty(), "This macro doesn't take any parameters");
	let mut global = GlobalContext::default();
	let mut module_inner = TokenStream::new();
	let mut local = LocalContext::default();

	for category in schema::decode(include_str!("../../elements/elements.kdl")) {
		trans_category(category, &mut global, &mut local, &mut module_inner);
	}

	let LocalContext {
		value_to_enum,
		id_to_str,
	} = local;

	let GlobalContext {
		str_to_id,
		enum_of_enums,
		path: _,
	} = global;

	{
		let mut module = TokenStream::new();
		quote2::quote!(module, {
			mod generated {
				#![allow(missing_docs)]
				#module_inner

				/// Enum of all support BCD enums
				#[derive(Clone, Copy, Debug, PartialEq, Eq)]
				pub enum Enum {
					#enum_of_enums
				}

				impl Enum {
					pub(super) fn __for_element_value(
						id:    ::core::primitive::u32,
						type_: crate::object::typing::ObjectType,
						value: ::core::primitive::u64,
					) -> ::core::option::Option<Self> {
						let [type_object, type_subtype, type_application] = decompose_type(type_);
						let mut ret = None;
						#value_to_enum
						ret
					}
				}

				pub(super) fn __str_to_id(string: &::core::primitive::str)
					-> ::core::option::Option<::core::primitive::u32>
				{
					match string {
						#str_to_id
						_ => ::core::option::Option::None,
					}
				}

				pub(super) fn __id_to_str(id: ::core::primitive::u32, type_: crate::object::typing::ObjectType)
					-> ::core::option::Option<&'static ::core::primitive::str>
				{
					let [type_object, type_subtype, type_application] = decompose_type(type_);
					let mut ret = None;
					#id_to_str
					ret
				}

				pub(super) fn decompose_type(type_: crate::object::typing::ObjectType)
					-> [::core::primitive::u8; 3]
				{
					use crate::object::typing::ObjectType;

					let (obj, sub, app);
					match type_ {
						ObjectType::Application { app_type, image } => {
							obj = 1;
							sub = image as u8;
							app = app_type as u8;
						},
						ObjectType::Inherit(type_) => {
							obj = 2;
							sub = type_ as u8;
							app = 0;
						},
						ObjectType::Device => {
							obj = 3;
							sub = 0;
							app = 0;
						}
					}

					[obj, sub, app]
				}
			}
		});

		module.into()
	}
}

fn trans_category(
	category: Category,
	global: &mut GlobalContext,
	local: &mut LocalContext,
	module_out: &mut TokenStream,
) {
	let mut module_inner = TokenStream::new();
	let name = ident(&category.name);
	global.path.push(name.clone());

	let mut value_to_enum_inner = TokenStream::new();
	let mut id_to_str_inner = TokenStream::new();
	let mut nested = LocalContext::default();

	for subcategory in category.subcategories {
		trans_category(subcategory, global, &mut nested, &mut module_inner);
	}

	for enum_ in category.enums {
		trans_enum(
			enum_,
			&global.path,
			&mut module_inner,
			&mut global.enum_of_enums,
		);
	}

	for element in category.elements {
		trans_element(
			element,
			&mut module_inner,
			&mut global.str_to_id,
			&mut id_to_str_inner,
			&mut value_to_enum_inner,
		);
	}

	let filter_condition = requirements_to_condition(&category.requirements);
	{
		let local = &mut local.value_to_enum;
		let nested = &mut nested.value_to_enum;

		quote!(local, {
			if #filter_condition {
				#nested

				match id {
					#value_to_enum_inner
					_ => (),
				}
			}
		});
	}

	{
		let local = &mut local.id_to_str;
		let nested = &mut nested.id_to_str;
		quote!(local, {
			if #filter_condition {
				#nested

				match id {
					#id_to_str_inner
					_ => (),
				}
			}
		});
	}

	global.path.pop();
	quote!(module_out, {
		pub mod #name { #module_inner }
	});
}

fn trans_element(
	element: Element,
	module: &mut TokenStream,
	str_to_id: &mut TokenStream,
	id_to_str: &mut TokenStream,
	value_to_enum: &mut TokenStream,
) {
	let name = element
		.name
		.unwrap_or_else(|| format!("Unknown0x{}", element.id));

	let name_ident = ident(&name.TO_SHOUTY_SNEK_CASE());
	let element_id = Literal::u32_unsuffixed(element.id);

	quote!(id_to_str, {
		#element_id => ret = ::core::option::Option::Some(#name),
	});

	if let Some(enum_) = element.r#enum {
		let enum_ = if enum_.is_empty() {
			ident(&name)
		} else {
			ident(&enum_)
		};

		quote!(value_to_enum, {
			#element_id =>
				ret = ::core::convert::TryFrom::try_from(value)
					.ok()
					.map(Self::#enum_),
		});
	}

	str_matcher_patterns(name, str_to_id);
	quote!(str_to_id, {
		=> ::core::option::Option::Some(#element_id),
	});

	quote!(module, {
		pub const #name_ident: crate::elements::DynamicElement =
			crate::elements::DynamicElement(#element_id);
	});
}

fn requirements_to_condition(requirements: &[Requires]) -> TokenStream {
	let mut out = TokenStream::new();
	for requirement in requirements {
		let inner = quote(|ts| {
			for (id, cond) in [
				("type_object", requirement.object),
				("type_subtype", requirement.subtype),
				("type_application", requirement.application),
			] {
				if let Some(value) = cond {
					let id = Ident::new(id, Span::call_site());
					quote!(ts, {
						#id == #value,
					});
				}
			}
		});

		quote!(out, {
			[#inner].into_iter().all(::core::convert::identity) ||
		});
	}

	let final_bool = requirements.is_empty();
	quote!(out, { #final_bool });
	out
}

fn trans_enum(
	enum_: Enum,
	module_path: &[Ident],
	module: &mut TokenStream,
	enum_of_enums: &mut TokenStream,
) {
	let name = ident(&enum_.name);
	let raw_varants: Vec<_> = enum_
		.variants
		.iter()
		.map(|EnumVariant { name, value }| {
			let name = if name.starts_with(char::is_numeric) {
				format_ident!("_{name}")
			} else {
				ident(name)
			};

			let value = Literal::u64_unsuffixed(*value);
			(name, value)
		})
		.collect();

	let variants = quote(|ts| {
		for (name, value) in &raw_varants {
			quote!(ts, {
				#name = #value,
			});
		}
	});

	let try_dwords = quote(|ts| {
		for (name, value) in &raw_varants {
			quote!(ts, {
				#value => ::core::result::Result::Ok(Self::#name),
			});
		}
	});

	let module_path = quote(|ts| {
		for segment in module_path {
			quote!(ts, { #segment :: });
		}
	});

	quote!(enum_of_enums, {
		#name(#module_path #name),
	});

	quote!(module, {
		#[derive(Clone, Copy, Debug, PartialEq, Eq, ::strum::EnumString, ::strum::IntoStaticStr)]
		pub enum #name { #variants }

		impl ::core::convert::TryFrom<::core::primitive::u64> for #name {
			type Error = ();

			fn try_from(value: ::core::primitive::u64) -> ::core::result::Result<Self, Self::Error> {
				match value {
					#try_dwords
					_ => ::core::result::Result::Err(()),
				}
			}
		}

		impl ::core::convert::From<#name> for ::core::primitive::u64 {
			fn from(value: #name) -> Self {
				value as ::std::primitive::u64
			}
		}
	});
}

#[derive(Default)]
struct GlobalContext {
	str_to_id: TokenStream,
	enum_of_enums: TokenStream,
	path: Vec<Ident>,
}

#[derive(Default)]
struct LocalContext {
	value_to_enum: TokenStream,
	id_to_str: TokenStream,
}

fn str_matcher_patterns(name: String, out: &mut TokenStream) {
	let kebab = name.to_kebab_case();
	let snek = name.TO_SHOUTY_SNEK_CASE();

	for variant in [
		Literal::string(&name),
		Literal::string(&kebab),
		Literal::string(&snek),
	]
	.into_iter()
	{
		quote!(out, {
			|#variant
		});
	}
}

fn ident(string: &str) -> Ident {
	Ident::new(string, Span::mixed_site())
}
