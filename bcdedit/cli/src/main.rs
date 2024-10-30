use {bcdedit::Bcd, clap::Parser as _, hivex::OpenFlags};

fn main() {
	let cli = bcdedit_cli::Cli::parse();
	let hive = hivex::Hive::open(cli.file, OpenFlags::empty()).unwrap();
	let bcd = Bcd::from_hive(hive).unwrap();
	for handle in bcd.objects().into_vec() {
		eprintln!("{:?}", bcd.object(handle).unwrap());
	}
}
