use knus::Decode;

#[derive(Decode, Debug)]
pub struct Category {
	#[knus(argument)]
	pub name: String,
	#[knus(children(name="enum"))]
	pub enums: Vec<Enum>,
	#[knus(children(name="element"))]
	pub elements: Vec<Element>,
	#[knus(children(name="category"))]
	pub subcategories: Vec<Self>,
}

#[derive(Decode, Debug)]
pub struct Enum {
	#[knus(argument)]
	pub name: String,
	#[knus(children)]
	pub variants: Vec<EnumVariant>,
}

#[derive(Decode, Debug)]
pub struct EnumVariant {
    #[knus(node_name)]
    pub name: String,
	#[knus(argument)]
	pub value: u64,
}

#[derive(Decode, Debug)]
pub struct Element {
	#[knus(argument)]
	pub id: u32,
	#[knus(property)]
	pub name: Option<String>,
	#[knus(property)]
	pub r#enum: Option<String>,
}

pub fn decode(content: &str) -> Vec<Category> {
	match knus::parse("<spec>", &content) {
		Ok(spec) => spec,
		Err(e) => {
			panic!("{:?}", miette::Report::new(e))
		}
	}
}
