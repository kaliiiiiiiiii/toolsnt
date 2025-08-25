use git2::{Repository, Status, StatusOptions};
use std::{
	env::var,
	fs, io,
	path::{Path, PathBuf},
	process::{Command, Output, Stdio},
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

fn cmd(cmd: &str, args: Vec<String>, cwd: &PathBuf, _msystem: &str) -> io::Result<Vec<u8>> {
	let shell = r"C:\msys64\usr\bin\bash.exe";

	// Run command
	let output: Output;
	let cmd_str: String;
	#[cfg(windows)]
	{
		cmd_str = format!(
			"{} {}",
			cmd,
			args.iter()
				.map(|arg| {
					if arg.contains(' ') {
						format!("\"{}\"", arg.replace("\"", "\\\""))
					} else {
						arg.clone()
					}
				})
				.collect::<Vec<_>>()
				.join(" ")
		);
		output = Command::new(shell)
			.arg("-lc")
			.arg(&cmd_str.replace("\\", "/"))
			//TODO: propperly choose https://github.com/ebiggers/wimlib/blob/cd2a5e5d2e95c36e81d09077d06ad136f7d24950/tools/windows-build.sh#L56-L94
			.env("MSYSTEM", _msystem)
			.env("CHERE_INVOKING", "1") // optional: avoids changing directories
			// .env("MSYS2_PATH_TYPE", "inherit") // optional: inherit Windows PATH
			.stdin(Stdio::null())
			.current_dir(cwd)
			.output()?;
	}

	#[cfg(not(windows))]
	{
		cmd_str = format!("{} {}", cmd, args.join(" "));
		output = Command::new(cmd)
			.args(args)
			.stdin(Stdio::null())
			.current_dir(cwd)
			.output()?;
	}

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

fn get_target() -> Result<(&'static str, &'static str, &'static str), String> {
	let arch =
		var("CARGO_CFG_TARGET_ARCH").map_err(|e| format!("Failed to read target arch: {e}"))?;

	match arch.as_str() {
		"i686" => Ok(("i686", "MINGW32", "mingw-w64-i686-gcc")), // clang32 not supported anymore by mysys2
		"x86_64" => Ok(("x86_64", "MINGW64", "mingw-w64-x86_64-gcc")), // or ("x86_64", "CLANG64", "mingw-w64-clang-x86_64-clang")
		"aarch64" => Ok(("aarch64", "CLANGARM64", "external")), // see https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/tools/windows-build.sh#L78-L89
		other => Err(format!("Unsupported arch: {other}")),
	}
}

#[cfg(windows)]
fn first_dir(path: &PathBuf) -> Option<PathBuf> {
	if !path.exists() {
		return None;
	}

	let mut dirs: Vec<PathBuf> = fs::read_dir(path)
		.ok()?
		.filter_map(|entry| entry.ok())
		.map(|entry| entry.path())
		.filter(|path| path.is_dir())
		.collect();

	dirs.sort();
	dirs.into_iter().next()
}

#[cfg(windows)]
pub fn find_mysys_include(sysenv: &str, mingw_host: &str, mingw_target: &str) -> Option<PathBuf> {
	let include_dir: PathBuf;
	let msys2_root = PathBuf::from("C:\\msys64");
	if mingw_target.contains("gcc") {
		let gcc_base_path = msys2_root
			.join(sysenv.to_lowercase())
			.join("lib\\gcc")
			.join(mingw_host);
		let version_dir = first_dir(&gcc_base_path)?;
		include_dir = version_dir.join("include");
	} else if mingw_target.contains("clang") {
		let clang_root = msys2_root.join(sysenv.to_lowercase()).join("lib\\clang");
		include_dir = first_dir(&clang_root)?.join("include")
	} else {
		return None;
	}

	if include_dir.exists() {
		Some(include_dir)
	} else {
		None
	}
}

/// Build and set linking instructions
fn bundled() -> Result<(), Box<dyn std::error::Error>> {
	let cargo_target_dir: PathBuf;
	#[cfg(windows)]
	{
		if var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc" {
			todo!("Building wimlib on windows for windows msvc isn't implemented yet. Needs https://stackoverflow.com/a/69293718/20443541");
		}
		#[cfg(windows)]
		{
			println!("cargo:warning=Building wimlib on windows for windows links libwim-15.dll dynamically");
			cargo_target_dir = PathBuf::from(
			var("CARGO_TARGET_DIR").expect("This crate requires CARGO_TARGET_DIR to be set for building for windows on windows. This is required for corretly placing libwim-15.dll"))
			.join(var("TARGET").unwrap())
			.join(var("PROFILE").unwrap())
			.join("libwim-15.dll");
		}
	}
	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let commitsha = commitsha(&manifest_dir.join("wimlib"));

	let (arch, sysenv, mingw_target) = get_target()?;

	let wimlib_src = &OUT_DIR.join("wimlib");

	// output directories

	let include = &OUT_DIR.join("include");
	fs::create_dir_all(include)?;

	let mut extra_bindgen_args: Vec<String> = vec![];
	let mut bindgen_target: String = var("CARGO_CFG_TARGET_ARCH").unwrap();

	// create clean workspace
	// git_clean(&manifest_dir.join("wimlib"))?;
	if wimlib_src.is_dir() {
		fs::remove_dir_all(wimlib_src)?;
	}
	copy_dir_all(manifest_dir.join("wimlib"), wimlib_src)?;

	// get wimlib version
	let bversion: Vec<u8>;
	bversion = cmd(
		wimlib_src
			.join("tools/get-version-number.sh")
			.to_str()
			.unwrap(),
		vec![],
		wimlib_src,
		sysenv,
	)?;
	let version = std::str::from_utf8(&bversion)?.trim_start().trim_end();

	println!(
		"cargo:warning=Building wimlib version: {} from {} at {}",
		version,
		get_repo_url(manifest_dir.join("wimlib")),
		commitsha
	);
	println!("cargo:rustc-env=LIB_VERSION={}", version);

	// bootstrap
	cmd(
		&wimlib_src.join("bootstrap").to_str().unwrap(),
		vec![],
		wimlib_src,
		sysenv,
	)?;

	// build for windows build target
	if var("CARGO_CFG_TARGET_OS")? == "windows" {
		// autoreconf
		// based on https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/tools/windows-build.sh#L201-L226
		let mingw_host = format!("{arch}-w64-mingw32");
		let mut args: Vec<String> = vec![
			"--without-fuse".to_string(),
			"--enable-shared".to_string(), // needed for windows-build.sh to succeed
			format!("--host={mingw_host}"),
			// 	"CFLAGS=-D_POSIX -D_POSIX_THREAD_SAFE_FUNCTIONS -DUNICODE -D_UNICODE -D_CRT_NON_CONFORMING_SWPRINTFS -D__MINGW_USE_VC2005_COMPAT -D_WIN32_WINNT=0x0600".to_string(), // Windows Vista or later
			//	"CCFLAGS=-D_POSIX -D_POSIX_THREAD_SAFE_FUNCTIONS -DUNICODE -D_UNICODE -D_CRT_NON_CONFORMING_SWPRINTFS -D__MINGW_USE_VC2005_COMPAT -D_WIN32_WINNT=0x0600".to_string(),
		];

		#[cfg(not(windows))] // we need to use dylib for windows on windows
		args.push("--enable-static".to_string()); // we want to link statically)

		if !cfg!(feature = "sys-ntfs-3g") || std::env::var("DOCS_RS").is_ok() {
			args.push("--without-ntfs-3g".to_string());
		}
		cmd(
			&wimlib_src.join("configure").to_string_lossy(),
			args,
			wimlib_src,
			sysenv,
		)?;

		// run windows-build.sh
		let buildscript = wimlib_src.join("tools/windows-build.sh");
		#[allow(unused_mut)] // not mut on linux
		let mut args: Vec<String> = vec![format!("--arch={arch}"), "--skip-configure".to_string()];

		// https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/tools/windows-build.sh#L122-L125
		#[cfg(windows)]
		args.push("--install-prerequisites".to_string());

		cmd(&buildscript.to_string_lossy(), args, wimlib_src, sysenv)?;

		// copy output files to include and lib
		fs::copy(
			wimlib_src.join("include/wimlib.h"),
			include.join("wimlib.h"),
		)?;

		#[cfg(windows)]
		fs::copy(wimlib_src.join(".libs/libwim-15.dll"), cargo_target_dir)
			.expect("copying .libs/libwim-15.dll failed");

		println!(
			"cargo:rustc-link-search=native={}",
			wimlib_src.join(".libs").to_string_lossy()
		);
		#[cfg(windows)]
		println!("cargo:rustc-link-lib=dylib=wim");
		#[cfg(not(windows))]
		println!("cargo:rustc-link-lib=static=wim");

		extra_bindgen_args.push(format!("-fms-extensions"));
		extra_bindgen_args.push(format!("-fdeclspec"));
		extra_bindgen_args.push(format!("-D_WIN32"));
		extra_bindgen_args.push(format!(
			"-I{}",
			wimlib_src.join("include").to_str().unwrap()
		));
		bindgen_target = format!("{arch}-pc-windows-gnu");

		#[cfg(windows)]
		{
			if let Some(mysys_include) = find_mysys_include(sysenv, &mingw_host, &mingw_target) {
				extra_bindgen_args.push(format!("-I{}", mysys_include.to_string_lossy()));
			} else {
				println!(
					"cargo:warning=System include might be missing, not resolved, to implement"
				);
			}
		}
		#[cfg(not(windows))]
		println!("cargo:warning=System include might be missing, not resolved, to implement");
	} else {
		// not building on windows
		println!("cargo:warning=System include might be missing, not resolved, to implement");
		let mut config = autotools::Config::new(wimlib_src);
		config.without("fuse", None).disable_shared();

		if !cfg!(feature = "sys-ntfs-3g") || std::env::var("DOCS_RS").is_ok() {
			config.without("ntfs-3g", None);
		}

		config.build();
		println!(
			"cargo:rustc-link-search=native={}",
			OUT_DIR.join("lib").to_string_lossy()
		);
		println!("cargo:rustc-link-lib=static=wim");
	}

	println!(
		"cargo:rerun-if-changed={}",
		manifest_dir.join("wimlib").to_string_lossy()
	);
	let builder = bindgen::builder()
		.header(OUT_DIR.join("include/wimlib.h").to_string_lossy())
		.clang_arg(format!("--target={bindgen_target}"))
		.clang_args(extra_bindgen_args);

	generate_bindings(builder)?;
	Ok(())
}
