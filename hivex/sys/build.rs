use {
	const_str::hex,
	flate2::bufread::GzDecoder,
	sha2::{Digest, Sha512},
	std::{
		env::var,
		fs::File,
		io::{BufReader, Seek},
		path::PathBuf,
		sync::LazyLock,
	},
};

static OUT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
	PathBuf::from(var("OUT_DIR").expect("Expected OUT_DIR to exist inside build context"))
});

fn main() -> Result<(), Box<dyn std::error::Error>> {
	if cfg!(feature = "bundled") {
		bundled()
	} else {
		system()
	}
}

fn generate_bindings(builder: bindgen::Builder) -> Result<(), Box<dyn std::error::Error>> {
	let bindings = builder
		.allowlist_item("hivex?_.*")
		.allowlist_item("HIVEX_.*")
		.allowlist_function("free")
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		.generate()?;

	let out_path = OUT_DIR.join("bindings.rs");
	bindings.write_to_file(out_path)?;
	Ok(())
}

/// Set linking and generating instructions for system library
fn system() -> Result<(), Box<dyn std::error::Error>> {
	let lib = pkg_config::probe_library("hivex")?;
	println!("cargo::rustc-env=LIB_VERSION={}", lib.version);

	generate_bindings(bindgen::builder().header_contents("bindings.h", "#include <hivex.h>"))
}

/// Build and set linking instructions
fn bundled() -> Result<(), Box<dyn std::error::Error>> {
	const VERSION: &str = "1.3.24";
	let hash = &hex!("4b9be259e0359344aee2dce1e4df56d928b0e429abcc099479ba95b2940fb80cd285f22e6a914902bcc716e8b4b528f204bea10977913fc701ae45aacb66669b")[..];

	println!(
		"{}",
		const_str::concat!("cargo::rustc-env=LIB_VERSION=", VERSION)
	);

	let src_dir = validate_and_extract(const_str::concat!("hivex-", VERSION), hash)?;
	autotools::Config::new(src_dir)
		.disable("ocaml", None)
		.disable("perl", None)
		.disable("python", None)
		.disable("ruby", None)
		.enable("year2038", None)
		.disable_shared()
		.build();

	println!(
		"cargo:rustc-link-search=native={}",
		OUT_DIR.join("lib").display()
	);
	println!("cargo:rustc-link-lib=static=hivex");

	generate_bindings(bindgen::builder().header(OUT_DIR.join("include/hivex.h").to_string_lossy()))
}

fn validate_and_extract(name: &str, hash: &[u8]) -> std::io::Result<PathBuf> {
	let path = PathBuf::from(format!("{name}.tar.gz"));
	let mut reader = BufReader::new(File::open(path)?);

	{
		let mut sha512 = Sha512::new();
		std::io::copy(&mut reader, &mut sha512)?;
		assert_eq!(&sha512.finalize()[..], hash);

		reader.rewind()?;
	}

	let mut archive = tar::Archive::new(GzDecoder::new(reader));
	archive.unpack(&*OUT_DIR)?;
	Ok(OUT_DIR.join(&name))
}
