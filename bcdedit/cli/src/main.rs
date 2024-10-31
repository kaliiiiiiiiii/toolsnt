use {
	bcdedit::{Bcd, StoreFlags},
	bcdedit_cli::{ApplicationError, Cli, Ops},
	clap::Parser as _,
	error_stack::{Result, ResultExt},
};

fn main() -> Result<(), ApplicationError> {
	let cli = bcdedit_cli::Cli::parse();
	match cli.ops {
		Ops::Init => init_bcd(&cli),
		Ops::Element { .. } => Ok(()),
	}
}

fn init_bcd(cli: &Cli) -> Result<(), ApplicationError> {
	Bcd::create(&cli.store, StoreFlags::empty(), hivex::OpenFlags::empty())
		.change_context(ApplicationError::Init)?;

	eprintln!("BCD Store {:?} initialized", &cli.store);
	Ok(())
}
