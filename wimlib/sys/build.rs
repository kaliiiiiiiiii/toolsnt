use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	pkg_config::probe_library("wimlib")?;
	let bindings = bindgen::builder()
		.header_contents("wrapper.h", "#include <wimlib.h>")
		.allowlist_item("wimlib_.*")
		.allowlist_item("WIMLIB_.*")
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		.generate()?;

	let out_path = PathBuf::from(std::env::var("OUT_DIR")?).join("bindings.rs");
	bindings.write_to_file(out_path)?;

	Ok(())
}
