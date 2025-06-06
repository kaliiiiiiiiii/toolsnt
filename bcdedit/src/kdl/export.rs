//! Exporting BCD components to KDL documents

use {
	super::KdlFormat,
	crate::{
		object::{typing::ObjectType, Object},
		value::{
			device::{Device, DeviceFormat, Partition},
			Value,
		},
		Store, StoreFlags,
	},
	kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue},
};

pub fn store(store: &Store, doc: &mut KdlDocument) {
	KdlFormat::export(&store.flags(), doc);
	for &object_h in store.objects().iter() {
		let Ok(object) = store.object(object_h) else {
			continue;
		};

		KdlFormat::export(&object, doc);
	}
}

pub fn store_flags(flags: StoreFlags, doc: &mut KdlDocument) {
	let mut node = KdlNode::new("flags");
	let mut flag = |name, other| node.push((name, flags.contains(other)));

	flag("system", StoreFlags::SYSTEM);
	flag("treat-as-systme", StoreFlags::TREAT_AS_SYSTEM);

	doc.nodes_mut().push(node);
}

pub fn object(object: &Object<'_>, doc: &mut KdlDocument) {
	let mut node;

	let type_ = object.type_();
	match type_ {
		ObjectType::Application { image, app_type } => {
			node = KdlNode::new("Application");
			node.push(("type", <&str>::from(app_type)));
			node.push(("image", <&str>::from(image)));
		}
		ObjectType::Inherit(for_) => {
			node = KdlNode::new("Inherit");
			node.push(("for", <&str>::from(for_)));
		}
		ObjectType::Device => node = KdlNode::new("Device"),
	}

	let uuid = object.uuid();
	node.push(uuid.to_string());

	let children = node.ensure_children();
	for result in object.elements().with_values() {
		let Ok((key, value)) = result else {
			continue;
		};

		let mut node = match key.name(type_) {
			Some(name) => KdlNode::new(name),
			None => KdlNode::new(format!("id:{:08x}", key.as_raw())),
		};

		match value {
			Value::Device(val) => device_format(val, node.ensure_children()),
			Value::String(val) => node.push(val.to_string()),
			Value::Guid(val) => node.push(val.to_string()),
			Value::GuidList(vals) => node_extend(&mut node, vals.iter().map(ToString::to_string)),
			Value::Integer(val) => node.push(i128::from(val)),
			Value::Bool(val) => node.push(val),
			Value::IntegerList(vals) => {
				node_extend(&mut node, vals.into_vec().into_iter().map(i128::from))
			}
		};

		children.nodes_mut().push(node);
	}

	doc.nodes_mut().push(node);
}

fn device_format(device_format: DeviceFormat, doc: &mut KdlDocument) {
	let nodes = doc.nodes_mut();

	{
		let mut node = KdlNode::new("additional-options");
		node.push(device_format.additional_options.to_string());
	}

	format_device(device_format.device, nodes);

	fn format_device(device: Device, nodes: &mut Vec<KdlNode>) {
		fn a_partition<In, IntoValue>(
			nodes: &mut Vec<KdlNode>,
			type_tag: &str,
			conv_f: impl Fn(In) -> IntoValue,
			disk: In,
			partition: In,
		) where
			IntoValue: Into<KdlValue>,
		{
			let mut node = KdlNode::new("Partition");
			node.push(("type", type_tag));
			node.push(("disk", conv_f(disk)));
			node.push(("partition", conv_f(partition)));
			nodes.push(node);
		}

		match device {
			Device::Partition(Partition::Mbr { partition, disk }) => {
				a_partition(nodes, "mbr", i128::from, partition, disk)
			}
			Device::Partition(Partition::Gpt { partition, disk }) => {
				a_partition(nodes, "gpt", ToString::to_string, &partition, &disk)
			}
			Device::File(file) => {
				format_device(*file.device, nodes);
				let mut node = KdlNode::new("File");
				node.push(file.path.to_string());
			}
			Device::Ramdisk(ramdisk) => {
				format_device(*ramdisk.file.device, nodes);
				let mut node = KdlNode::new("Ramdisk");
				node.push(ramdisk.file.path.to_string());
			}
		}
	}
}

fn node_extend(node: &mut KdlNode, items: impl IntoIterator<Item = impl Into<KdlValue>>) {
	node.entries_mut()
		.extend(items.into_iter().map(KdlEntry::new));
}

#[cfg(test)]
mod tests {
	use {super::*, hivex::OpenFlags};

	#[test]
	fn store() {
		let store = Store::open("Test-Store", OpenFlags::empty()).unwrap();
		let mut doc = KdlDocument::default();
		store.export(&mut doc);
		doc.autoformat();
		panic!("{doc}");
	}
}
