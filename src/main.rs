use std::error;
use std::fs;
use std::fs::File;
use std::io::Read;

extern crate structopt;
use structopt::StructOpt;

extern crate toml_edit;
use toml_edit::{value, Document};

extern crate reqwest;

extern crate serde_json;
use serde_json::Value;

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
        handle_file(file, opt.unsafe_file_updates)?;
    }

    Ok(())
}

fn handle_file(filename: &str, unsafe_file_updates: bool) -> Result<(), Box<error::Error>> {
    println!("Reading file: {}", filename);

    let mut contents = String::new();
    {
        let mut f = File::open(filename)?;
        f.read_to_string(&mut contents)
            .expect("Something went wrong reading the file.");
    }

    let new_contents = update_toml(&contents)?;
    if new_contents == contents {
        return Ok(());
    }

    if !unsafe_file_updates {
        let filename_old = filename.to_string() + ".old";
        let _ = fs::remove_file(&filename_old);
        fs::copy(filename, filename_old)?;
    }

    fs::write(filename, new_contents)?;
    Ok(())
}

#[test]
fn test_update_toml() {
    let toml = r#"
[package]
version = "0.1.0"

[dependencies]
structopt = "0.2"
toml_edit = "0.1.3"
    "#;

    let expected = r#"
[package]
version = "0.1.0"

[dependencies]
structopt = "0.2.15"
toml_edit = "0.1.3"
    "#;

    let result = update_toml(toml).unwrap();
    assert_eq!(result, expected);
}

fn update_toml(toml: &str) -> Result<String, Box<error::Error>> {
    let mut doc = toml.parse::<Document>()?;

    let mut updates: Vec<(String, String)> = Vec::new();
    {
        let section = &doc["dependencies"].as_table();
        for key in section {
            for (the_crate, local_version) in key.iter() {
                let local_version = local_version.as_value().unwrap();
                let local_version = match local_version.as_str() {
                    Some(v) => v.trim(),
                    None => {
                        println!("** Error: Can not parse {}", &local_version);
                        continue;
                    }
                };

                println!("\tFound: {}", the_crate);
                println!("\t\tLocal version:  {}", local_version);

                let online_version = lookup_latest_version(&the_crate)?;
                println!("\t\tOnline version: {}", &online_version);

                if local_version != online_version {
                    updates.push((the_crate.to_string(), online_version));
                }
            }
        }
    }

    for (the_crate, version) in updates {
        doc["dependencies"][&the_crate] = value(version);
    }
    Ok(doc.to_string())
}

fn lookup_latest_version(crate_name: &str) -> Result<String, Box<error::Error>> {
    let uri = "https://crates.io/api/v1/crates/".to_string() + crate_name;

    let mut http_res = reqwest::get(&uri)?;
    let mut http_body = String::new();
    http_res.read_to_string(&mut http_body)?;

    let json_doc: Value = serde_json::from_str(&http_body)?;
    let mut version: String = json_doc["versions"][0]["num"].to_string();

    if version.starts_with('"') {
        version = version.get(1..).unwrap().to_string();
    }
    if version.ends_with('"') {
        let end = version.len() - 1;
        version = version.get(..end).unwrap().to_string();
    }

    Ok(version)
}
