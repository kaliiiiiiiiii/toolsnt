use {clap::Parser, std::path::PathBuf, uuid::Uuid};

/// Command line tool for manipulating Windows Boot Configuration Data
#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
pub struct Cli {
	/// Path to BCD hive to operate on
	#[arg(short, long)]
	pub store: PathBuf,

	#[command(subcommand)]
	pub ops: Ops,
}

#[derive(Parser, Debug)]
pub enum Ops {
	/// Initialize a new BCD Store
	#[command(name = "--init", short_flag = 'I')]
	Init,
	/// Manipulate BCD objects
	#[command(name = "--object", short_flag = 'O')]
	Object {
		uuid: Uuid,
		#[command(subcommand)]
		ops: Option<ObjectOps>,
	},
}

#[derive(Parser, Debug)]
pub enum ObjectOps {}
