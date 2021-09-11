extern crate libdr;

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

    /// Use exact version compare instead of SemVer compare for version comparison.
    /// If this option is given, then even minor version updates will
    /// be recognized as new versions.
    /// Usually exact compare is not needed, because Cargo recognizes compatible SemVer
    /// versions and uses the latest compatible version anyway.
    #[structopt(short, long)]
    exact: bool,

    /// Allow update to yanked versions.
    #[structopt(short, long)]
    yanked: bool,

    /// Allow update to pre-release (-beta or -rc) versions.
    #[structopt(short, long)]
    pre_release: bool,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let opt = Opt::from_args();

    for file in &opt.toml_files {
        libdr::update_toml_file(file,
                                opt.unsafe_file_updates,
                                !opt.exact,
                                opt.yanked,
                                opt.pre_release)?;
    }

    Ok(())
}
