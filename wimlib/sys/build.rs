use git2::{Repository, Status, StatusOptions};
use std::{
	env::var,
	fs, io,
	path::{Path, PathBuf},
	process::{Command, Stdio},
	sync::LazyLock,
};

static OUT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
	PathBuf::from(var("OUT_DIR").expect("Expected OUT_DIR to exist inside build context"))
});

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let is_docs_rs = std::env::var("DOCS_RS").is_ok();

	if cfg!(feature = "bundled") || is_docs_rs {
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

fn get_repo_url(repo_path: PathBuf) -> String {
	let repo = match Repository::open(repo_path) {
		Ok(r) => r,
		Err(_) => return "unknown".to_string(),
	};

	// Force the string to be owned immediately to avoid temporary borrow issues
	if let Ok(remote) = repo.find_remote("origin") {
		if let Some(url) = remote.url() {
			return url.to_string(); // OWNED string
		}
	}

	"unknown".to_string()
}

fn commitsha(repo_path: &PathBuf) -> String {
	let repo = Repository::open(repo_path).unwrap();

	let head_commit = repo.head().and_then(|h| h.peel_to_commit()).unwrap();

	return head_commit.id().to_string().to_string(); // SHA
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
	fs::create_dir_all(&dst)?;
	for entry in fs::read_dir(src)? {
		let entry = entry?;
		let ty = entry.file_type()?;
		if ty.is_dir() {
			copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
		} else {
			fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
		}
	}
	Ok(())
}

fn git_clean(repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
	let repo = Repository::open(repo_path)?;

	let mut opts = StatusOptions::new();
	opts.include_ignored(true)
		.include_untracked(true)
		.recurse_untracked_dirs(true);

	let statuses = repo.statuses(Some(&mut opts))?;

	for entry in statuses.iter() {
		let s = entry.status();
		if s.contains(Status::WT_NEW) || s.contains(Status::IGNORED) {
			if let Some(path) = entry.path() {
				let full_path = Path::new(repo_path.to_str().unwrap()).join(path);
				if full_path.is_dir() {
					fs::remove_dir_all(&full_path)?;
				} else if full_path.is_file() {
					fs::remove_file(&full_path)?;
				}
			}
		}
	}

	Ok(())
}

fn msys2_cmd(cmd: &String, args: Vec<String>, cwd: &PathBuf) -> io::Result<Vec<u8>> {
	// Determine target
	// let target = var("TARGET").unwrap_or_default();
	let shell = r"C:\msys64\usr\bin\bash.exe";

	// Build a single safe command string
	let cmd_str = format!("{} {}", cmd, args.join(" "));

	// Run command
	let output = Command::new(shell)
		.arg("-lc")
		.arg(&cmd_str)
		//TODO: propperly choose https://github.com/ebiggers/wimlib/blob/cd2a5e5d2e95c36e81d09077d06ad136f7d24950/tools/windows-build.sh#L56-L94
		.env("MSYSTEM", "CLANG64")
		.env("CHERE_INVOKING", "1") // optional: avoids changing directories
		// .env("MSYS2_PATH_TYPE", "inherit") // optional: inherit Windows PATH
		.stdin(Stdio::null())
		.current_dir(cwd)
		.output()?;

	if output.status.success() {
		Ok(output.stdout)
	} else {
		eprintln!(
			"Command {} {} failed with status {:?}:",
			shell,
			cmd_str,
			output.status.code()
		);
		eprintln!("{}", String::from_utf8_lossy(&output.stdout));
		eprintln!("{}", String::from_utf8_lossy(&output.stderr));
		Err(io::Error::new(
			io::ErrorKind::Other,
			"Command execution failed",
		))
	}
}
fn cmd(cmd: &str, args: Vec<String>, cwd: &PathBuf) -> io::Result<Vec<u8>> {
	// Build command with arguments
	let mut command = Command::new(cmd);
	command.args(args);

	// Run command
	let output = command.stdin(Stdio::null()).current_dir(cwd).output()?;

	if output.status.success() {
		Ok(output.stdout)
	} else {
		eprintln!(
			"Command {} failed with status {:?}:",
			cmd,
			output.status.code()
		);
		eprintln!("{}", String::from_utf8_lossy(&output.stdout));
		eprintln!("{}", String::from_utf8_lossy(&output.stderr));
		Err(io::Error::new(
			io::ErrorKind::Other,
			"Command execution failed",
		))
	}
}

/// Build and set linking instructions
fn bundled() -> Result<(), Box<dyn std::error::Error>> {
	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	let wimlib_src = &OUT_DIR.join("wimlib");
	let commitsha = commitsha(&manifest_dir.join("wimlib"));
	let commitshaf = &OUT_DIR.join("wimlibcommitsha");

	// cache wimlib build
	if !(wimlib_src.exists()
		&& commitshaf.is_file()
		&& (fs::read_to_string(commitshaf).unwrap() == commitsha))
	{
		git_clean(&manifest_dir.join("wimlib"))?;
		copy_dir_all(manifest_dir.join("wimlib"), wimlib_src)?;
		

		let version_script = &wimlib_src
			.join("tools/get-version-number.sh")
			.to_str()
			.unwrap()
			.replace("\\", "/");

		// identify wimlib version
		let bversion: Vec<u8>;
		if cfg!(target_os = "windows") {
			bversion = msys2_cmd(version_script, vec![], wimlib_src)?;
		} else {
			bversion = cmd(version_script, vec![], wimlib_src)?;
		}
		let version: String = std::str::from_utf8(&bversion)?
			.trim_start()
			.trim_end()
			.to_string();

		println!(
			"cargo:warning=Building wimlib version: {} from {} at {}",
			version,
			get_repo_url(manifest_dir.join("wimlib")),
			commitsha
		);
		println!("cargo:rustc-env=LIB_VERSION={}", version);

		if cfg!(target_os = "windows") {
			// bootstrap
			msys2_cmd(
				&wimlib_src
					.join("bootstrap")
					.to_str()
					.unwrap()
					.replace("\\", "/"),
				vec![],
				wimlib_src,
			)?;

			// autoreconf
			// won't have an effect due to https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/tools/windows-build.sh#L201-L226
			let mut args: Vec<String> =
				vec!["--without-fuse".to_string(), "--disable-shared".to_string()];
			if !cfg!(feature = "sys-ntfs-3g") || std::env::var("DOCS_RS").is_ok() {
				args.push("--without-ntfs-3g".to_string());
			}
			msys2_cmd(
				&wimlib_src
					.join("configure")
					.to_str()
					.unwrap()
					.replace("\\", "/"),
				args,
				wimlib_src,
			)?;

			// actuall build
			let buildscript = wimlib_src.join("tools/windows-build.sh");
			msys2_cmd(
				&buildscript.to_str().unwrap().replace("\\", "/"),
				vec!["--install-prerequisites".to_string()],
				wimlib_src,
			)?;

			// https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/tools/windows-build.sh#L155
			let out_bin = wimlib_src.join(format!("wimlib-{}-windows-x86_64-bin", version));

			fs::create_dir_all(OUT_DIR.join("include"))?;
			fs::copy(
				out_bin.join("devel/wimlib.h"),
				OUT_DIR.join("include/wimlib.h"),
			)?;
			fs::copy(
				manifest_dir.join("include/stdbool.h"),
				OUT_DIR.join("include/stdbool.h"),
			)?;
			copy_dir_all(out_bin, OUT_DIR.join("lib"))?;
		} else {
			let mut config = autotools::Config::new(wimlib_src);
			config.without("fuse", None).disable_shared();

			if !cfg!(feature = "sys-ntfs-3g") || std::env::var("DOCS_RS").is_ok() {
				config.without("ntfs-3g", None);
			}

			config.build();
		}
		fs::write(commitshaf, commitsha)?;
	}

	println!(
		"cargo:rerun-if-changed={}",
		OUT_DIR.join("include/wimlib.h").to_str().unwrap()
	);

	println!(
		"cargo:rustc-link-search=native={}",
		OUT_DIR.join("lib").display()
	);

	#[cfg(not(windows))]
	println!("cargo:rustc-link-lib=static=wim");
	#[cfg(windows)]
	println!("cargo:rustc-link-lib=dylib=wim-15");

	generate_bindings(
		bindgen::builder()
			.header(OUT_DIR.join("include/wimlib.h").to_string_lossy())
			.clang_arg(format!("-I{}", OUT_DIR.join("include").to_str().unwrap())),
	)
}
