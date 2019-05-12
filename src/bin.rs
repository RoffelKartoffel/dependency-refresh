extern crate drlib;

use std::error;

extern crate structopt;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "dependency-refresh",
    about = "A rust dependency version updater."
)]
struct Opt {
    #[structopt(raw(required = "true", min_values = "1"))]
    toml_files: Vec<String>,

    #[structopt(short = "u", long = "unsafe-file-updates")]
    unsafe_file_updates: bool,
}

fn main() -> Result<(), Box<error::Error>> {
    let opt = Opt::from_args();

    for file in &opt.toml_files {
        drlib::handle_file(file, opt.unsafe_file_updates)?;
    }

    Ok(())
}
