use {clap::Parser, std::path::PathBuf};

/// Command line tool for manipulating Windows Boot Configuration Data
#[derive(Parser, Debug)]
pub struct Cli {
	/// Path to BCD hive to operate on
	#[arg(short, long)]
	pub file: PathBuf,
}
