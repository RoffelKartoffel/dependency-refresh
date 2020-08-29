use std::error;
use std::fs;
use std::fs::File;
use std::io::Read;

extern crate toml_edit;
use toml_edit::Document;

extern crate reqwest;

extern crate serde_json;
use serde_json::Value;
use reqwest::header::USER_AGENT;

pub fn update_toml_file(
    filename: &str,
    unsafe_file_updates: bool,
) -> Result<(), Box<dyn error::Error>> {
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
reqwest = { version = "0.10.3", features = ["blocking"] }
structopt = "0.2"
toml_edit = "0.1.3"
    "#;

    let expected = r#"
[package]
version = "0.1.0"

[dependencies]
reqwest = { version = "0.10.8", features = ["blocking"] }
structopt = "0.3.18"
toml_edit = "0.2.0"
    "#;

    let result = update_toml(toml).unwrap();
    assert_eq!(result, expected);
}

fn update_toml(toml: &str) -> Result<String, Box<dyn error::Error>> {
    let doc = toml.parse::<Document>()?;

    let mut updates_crate = Vec::new();
    let mut updates_crate_version = Vec::new();

    let table = &doc["dependencies"].as_table().unwrap();
    for (the_crate, item) in table.iter() {
        println!("\tFound: {}", the_crate);

        let value = item.as_value().unwrap();
        if let Some(local_version) = value.as_str() {
            let local_version = local_version.trim();
            println!("\t\tLocal version:  {}", local_version);

            let online_version = lookup_latest_version(&the_crate)?;
            println!("\t\tOnline version: {}", &online_version);

            if local_version != online_version {
                updates_crate.push((the_crate.to_string(), toml_edit::value(online_version)));
            }
        }
        else if let Some(inline_table) =  value.as_inline_table() {
            if let Some(value) = inline_table.get("version") {
                if let Some(local_version) = value.as_str() {
                    let local_version = local_version.trim();
                    println!("\t\tLocal version:  {}", local_version);

                    let online_version = lookup_latest_version(&the_crate)?;
                    println!("\t\tOnline version: {}", &online_version);

                    if local_version != online_version {
                        updates_crate_version.push((the_crate.to_string(), toml_edit::value(online_version)));
                    }
                }
            }
        }
        else {
            println!("** Error: Can not parse {}", &value);
        }
    }

    let mut doc = doc;
    for (the_crate, version) in updates_crate {
        doc["dependencies"][&the_crate] = version;
    }
    for (the_crate, version) in updates_crate_version {
        doc["dependencies"][&the_crate]["version"] = version;
    }

    Ok(doc.to_string())
}

fn lookup_latest_version(crate_name: &str) -> Result<String, Box<dyn error::Error>> {

    const NAME: &'static str = env!("CARGO_PKG_NAME");
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const REPO: &'static str = env!("CARGO_PKG_REPOSITORY");
    let user_agent = format!("{} {} ( {} )", NAME, VERSION, REPO);

    let uri = format!("https://crates.io/api/v1/crates/{}", crate_name);

    let client = reqwest::blocking::Client::builder()
        .gzip(true)
        .build()?;
    let http_body = client.get(&uri)
        .header(USER_AGENT, &user_agent)
        .send()?
        .text()?;

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
