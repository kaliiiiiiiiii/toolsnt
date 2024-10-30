mod common;
use {
	common::TestHive,
	hivex::{value::Value, SetValueFlags},
};

#[test]
fn multistring() {
	let hive = TestHive::new(true).unwrap();
	let root = hive.node(hive.root().unwrap());

	let expected: Box<[&str]> = ["cat", "sus"].into();
	root.set_value(
		SetValueFlags::default(),
		c"nyan",
		Value::MultiSz(expected.clone()),
	)
	.unwrap();

	let Value::MultiSz(value) = hive.value(root.get_value(c"nyan").unwrap()).get().unwrap() else {
		panic!("Expected MultiSz");
	};

	for (expected, got) in expected.iter().zip(value) {
		assert_eq!(*expected, &got[..]);
	}
}

#[test]
fn string() {
	let hive = TestHive::new(true).unwrap();
	let root = hive.node(hive.root().unwrap());

	let expected = ":3";
	root.set_value(SetValueFlags::default(), c"nyan", Value::Sz(expected))
		.unwrap();

	let Value::Sz(value) = hive.value(root.get_value(c"nyan").unwrap()).get().unwrap() else {
		panic!("Expected string")
	};

	assert_eq!(expected, value.trim_end_matches('\0'));
}
