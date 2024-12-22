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
		.allowlist_item("wimlib_.*")
		.allowlist_item("WIMLIB_.*")
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		.generate()?;

	let out_path = OUT_DIR.join("bindings.rs");
	bindings.write_to_file(out_path)?;
	Ok(())
}

/// Set linking and generating instructions for system library
fn system() -> Result<(), Box<dyn std::error::Error>> {
	let lib = pkg_config::probe_library("wimlib")?;
	println!("cargo::rustc-env=LIB_VERSION={}", lib.version);

	generate_bindings(bindgen::builder().header_contents("bindings.h", "#include <wimlib.h>"))
}

/// Build and set linking instructions
fn bundled() -> Result<(), Box<dyn std::error::Error>> {
	const VERSION: &str = "1.14.4";
	let hash = &hex!("f3c25ee14fe849f452f004ce8137ef040410ea048555ae71180086f010858b6ed593c8881b805bac65f9ee878bf11661a7f17677c6c24e2c77149c35ee0cd853")[..];

	println!(
		"{}",
		const_str::concat!("cargo::rustc-env=LIB_VERSION=", VERSION)
	);

	let src_dir = validate_and_extract(const_str::concat!("wimlib-", VERSION), hash)?;
	let mut config = autotools::Config::new(src_dir);
	config.without("fuse", None).disable_shared();

	if !cfg!(feature = "sys-ntfs-3g") || std::env::var("DOCS_RS").is_ok() {
		config.without("ntfs-3g", None);
	}

	config.build();

	println!(
		"cargo:rustc-link-search=native={}",
		OUT_DIR.join("lib").display()
	);

	println!("cargo:rustc-link-lib=static=wim");

	generate_bindings(bindgen::builder().header(OUT_DIR.join("include/wimlib.h").to_string_lossy()))
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
