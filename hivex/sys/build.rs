use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let lib = pkg_config::probe_library("hivex")?;
	let bindings = bindgen::builder()
		.header_contents("wrapper.h", "#include <hivex.h>")
		.allowlist_item("hivex?_.*")
		.allowlist_item("HIVEX_.*")
		.allowlist_function("free")
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		.generate()?;

	let out_path = PathBuf::from(std::env::var("OUT_DIR")?).join("bindings.rs");
	bindings.write_to_file(out_path)?;

	println!("cargo::rustc-env=LIB_VERSION={}", lib.version);

	Ok(())
}
