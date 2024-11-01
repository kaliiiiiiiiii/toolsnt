use {
	bcdedit_cli::{cli::Cli, process_cli, ApplicationError},
	clap::Parser as _,
	error_stack::Result,
};

fn main() -> Result<(), ApplicationError> {
	let cli = Cli::parse();
	process_cli(cli)
}
