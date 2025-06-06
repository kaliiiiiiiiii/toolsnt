//! KDL Import / Export Facilities for BCD components

mod export;

use {
	crate::{object::Object, Store, StoreFlags},
	kdl::KdlDocument,
};

/// BCD component which can be represented as KDL document
pub trait KdlFormat {
	/// Export BCD component to a KDL document.
	///
	/// This function takes a mutable reference to already existing KDL
	/// document. In most cases you'd probably need an empty KDL document,
	/// which can be crated by [`KdlDocument::default`].
	fn export(&self, document: &mut KdlDocument);
}

impl KdlFormat for Store {
	fn export(&self, document: &mut KdlDocument) {
		export::store(self, document)
	}
}

impl KdlFormat for StoreFlags {
	fn export(&self, document: &mut KdlDocument) {
		export::store_flags(*self, document);
	}
}

impl KdlFormat for Object<'_> {
	fn export(&self, document: &mut KdlDocument) {
		export::object(self, document);
	}
}
