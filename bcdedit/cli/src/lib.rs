use {
	clap::{Parser, Parser as _},
	derive_more::{Display, Error},
	error_stack::Result,
	std::path::PathBuf,
};

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
	/// Manipulate BCD elements
	#[command(name = "--element", short_flag = 'E')]
	Element,
}

#[derive(Debug, Display, Error, PartialEq, Eq)]
pub enum ApplicationError {
	#[display("BCD initialization failed")]
	Init,
}
