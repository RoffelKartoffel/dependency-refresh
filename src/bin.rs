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
    #[structopt(required = true, min_values = 1)]
    toml_files: Vec<String>,

    #[structopt(short = "u", long = "unsafe-file-updates")]
    unsafe_file_updates: bool,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let opt = Opt::from_args();

    for file in &opt.toml_files {
        drlib::update_toml_file(file, opt.unsafe_file_updates)?;
    }

    Ok(())
}
