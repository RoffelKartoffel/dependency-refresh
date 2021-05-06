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

extern crate semver;
use semver::Version;
use semver::VersionReq;

#[derive(Debug)]
struct Error(String);

impl error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn update_toml_file(
    filename: &str,
    unsafe_file_updates: bool,
    use_semver: bool,
) -> Result<(), Box<dyn error::Error>> {
    println!("Reading file: {}", filename);

    let mut contents = String::new();
    {
        let mut f = File::open(filename)?;
        f.read_to_string(&mut contents)
            .expect("Something went wrong reading the file.");
    }

    let new_contents = update_toml(&contents, use_semver)?;
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
fn test_update_toml_semver() {
    let toml = r#"
[package]
version = "0.1.0"

[dependencies]
reqwest = { version = "0.10.3", features = ["blocking"] }
structopt = "0.3"

[dependencies.toml_edit]
version = "0.1.3"

[build-dependencies]
autocfg = "1.0.0"
    "#;

    let expected = r#"
[package]
version = "0.1.0"

[dependencies]
reqwest = { version = "0.11.3", features = ["blocking"] }
structopt = "0.3"

[dependencies.toml_edit]
version = "0.2.0"

[build-dependencies]
autocfg = "1.0.0"
    "#;

    let result = update_toml(toml, true).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_update_toml_exact() {
    let toml = r#"
[package]
version = "0.1.0"

[dependencies]
reqwest = { version = "0.10.3", features = ["blocking"] }
structopt = "0.3"

[dependencies.toml_edit]
version = "0.1.3"

[build-dependencies]
autocfg = "1.0.0"
    "#;

    let expected = r#"
[package]
version = "0.1.0"

[dependencies]
reqwest = { version = "0.11.3", features = ["blocking"] }
structopt = "0.3.21"

[dependencies.toml_edit]
version = "0.2.0"

[build-dependencies]
autocfg = "1.0.1"
    "#;

    let result = update_toml(toml, false).unwrap();
    assert_eq!(result, expected);

}

fn version_matches(local_version: &str,
                   online_version: &str,
                   use_semver: bool)
                   -> Result<bool, Box<dyn error::Error>> {
    if use_semver {
        let local_version_sem = match VersionReq::parse(local_version) {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(Error(format!("Failed to parse Cargo.toml version '{}': {}",
                                                 local_version, e)))),
        }?;
        let online_version_sem = match Version::parse(online_version) {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(Error(format!("Failed to parse online version '{}': {}",
                                                 online_version, e)))),
        }?;
        Ok(local_version_sem.matches(&online_version_sem))
    } else {
        Ok(*local_version == *online_version)
    }
}

fn check_version(updates_crate: &mut Vec<(String, String, String)>,
                 the_crate: &str,
                 local_version: &str,
                 use_semver: bool)
                 -> Result<(), Box<dyn error::Error>> {
    let local_version = local_version.trim().to_string();
    println!("\t\tLocal version:  {}", local_version);

    let online_version = lookup_latest_version(the_crate)?;
    println!("\t\tOnline version: {}", &online_version);

    if !version_matches(&local_version, &online_version, use_semver)? {
        updates_crate.push((the_crate.to_string(), local_version, online_version));
    }
    Ok(())
}

fn update_info(the_crate: &str,
               local_version: &str,
               online_version: &str) {
    println!("\tUpdating: {} {} => {}",
             the_crate,
             local_version,
             online_version);
}

fn update_toml_dep_table(doc: &mut Document,
                         table_name: &str,
                         use_semver: bool)
                         -> Result<(), Box<dyn error::Error>> {
    if let Some(table) = &doc[table_name].as_table() {
        let mut updates_crate = Vec::new();
        let mut updates_crate_version = Vec::new();

        for (the_crate, item) in table.iter() {
            println!("\tFound: {}", the_crate);

            if let Some(sub_table) = item.as_table() {
                if let Some(value) = sub_table.get("version") {
                    if let Some(local_version) = value.as_str() {
                        check_version(&mut updates_crate_version, the_crate, local_version, use_semver)?;
                    }
                }
            } else if let Some(value) = item.as_value() {
                if let Some(local_version) = value.as_str() {
                    check_version(&mut updates_crate, the_crate, local_version, use_semver)?;
                }
                else if let Some(inline_table) = value.as_inline_table() {
                    if let Some(value) = inline_table.get("version") {
                        if let Some(local_version) = value.as_str() {
                            check_version(&mut updates_crate_version, the_crate, local_version, use_semver)?;
                        }
                    }
                } else {
                    println!("** Error: Can not parse {}", value);
                }
            } else {
                println!("** Error: Item '{:?}' is neither table nor value.", item);
            }
        }

        for (the_crate, local_version, online_version) in updates_crate {
            update_info(&the_crate, &local_version, &online_version);
            doc[table_name][&the_crate] = toml_edit::value(online_version);
        }
        for (the_crate, local_version, online_version) in updates_crate_version {
            update_info(&the_crate, &local_version, &online_version);
            doc[table_name][&the_crate]["version"] = toml_edit::value(online_version);
        }
    }
    Ok(())
}

fn update_toml(toml: &str, use_semver: bool) -> Result<String, Box<dyn error::Error>> {
    let mut doc = toml.parse::<Document>()?;
    update_toml_dep_table(&mut doc, "dependencies", use_semver)?;
    update_toml_dep_table(&mut doc, "build-dependencies", use_semver)?;
    Ok(doc.to_string())
}

fn lookup_latest_version(crate_name: &str) -> Result<String, Box<dyn error::Error>> {

    const NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const REPO: &str = env!("CARGO_PKG_REPOSITORY");
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
