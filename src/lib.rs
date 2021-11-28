use std::error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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

#[derive(Clone)]
pub struct DepRefresh {
    unsafe_file_updates:    bool,
    use_semver:             bool,
    allow_yanked:           bool,
    allow_prerelease:       bool,
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
reqwest = { version = "0.11.6", features = ["blocking"] }
structopt = "0.3"

[dependencies.toml_edit]
version = "0.10.0"

[build-dependencies]
autocfg = "1.0.0"
    "#;

    let dr = DepRefresh::new(false, true, false, false);
    let result = dr.update_toml(toml, &Path::new("Cargo.toml")).unwrap();
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
reqwest = { version = "0.11.6", features = ["blocking"] }
structopt = "0.3.25"

[dependencies.toml_edit]
version = "0.10.0"

[build-dependencies]
autocfg = "1.0.1"
    "#;

    let dr = DepRefresh::new(false, false, false, false);
    let result = dr.update_toml(toml, &Path::new("Cargo.toml")).unwrap();
    assert_eq!(result, expected);
}

impl DepRefresh {
    pub fn new(unsafe_file_updates: bool,
               use_semver: bool,
               allow_yanked: bool,
               allow_prerelease: bool) -> DepRefresh {
        DepRefresh {
            unsafe_file_updates,
            use_semver,
            allow_yanked,
            allow_prerelease,
        }
    }

    fn version_matches(&self,
                       local_version: &str,
                       online_version: &str)
                       -> Result<bool, Box<dyn error::Error>> {
        if self.use_semver {
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

    fn check_version(&self,
                     updates_crate: &mut Vec<(String, String, String)>,
                     the_crate: &str,
                     local_version: &str)
                     -> Result<(), Box<dyn error::Error>> {
        let local_version = local_version.trim().to_string();
        println!("\t\tLocal version:  {}", local_version);

        let online_version = self.lookup_latest_version(the_crate)?;
        println!("\t\tOnline version: {}", &online_version);

        if !self.version_matches(&local_version, &online_version)? {
            updates_crate.push((the_crate.to_string(), local_version, online_version));
        }
        Ok(())
    }

    fn update_info(&self,
                   the_crate: &str,
                   local_version: &str,
                   online_version: &str) {
        println!("\tUpdating: {} {} => {}",
                 the_crate,
                 local_version,
                 online_version);
    }

    fn update_toml_dep_table(&self,
                             doc: &mut Document,
                             table_name: &str)
                             -> Result<(), Box<dyn error::Error>> {
        if let Some(table) = &doc[table_name].as_table() {
            let mut updates_crate = Vec::new();
            let mut updates_crate_version = Vec::new();

            for (the_crate, item) in table.iter() {
                println!("\tFound: {}", the_crate);

                if let Some(sub_table) = item.as_table() {
                    if let Some(value) = sub_table.get("version") {
                        if let Some(local_version) = value.as_str() {
                            self.check_version(&mut updates_crate_version, the_crate, local_version)?;
                        }
                    }
                } else if let Some(value) = item.as_value() {
                    if let Some(local_version) = value.as_str() {
                        self.check_version(&mut updates_crate, the_crate, local_version)?;
                    }
                    else if let Some(inline_table) = value.as_inline_table() {
                        if let Some(value) = inline_table.get("version") {
                            if let Some(local_version) = value.as_str() {
                                self.check_version(&mut updates_crate_version, the_crate, local_version)?;
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
                self.update_info(&the_crate, &local_version, &online_version);
                doc[table_name][&the_crate] = toml_edit::value(online_version);
            }
            for (the_crate, local_version, online_version) in updates_crate_version {
                self.update_info(&the_crate, &local_version, &online_version);
                doc[table_name][&the_crate]["version"] = toml_edit::value(online_version);
            }
        }
        Ok(())
    }

    fn update_toml(&self, toml: &str, toml_filename: &Path) -> Result<String, Box<dyn error::Error>> {
        let mut doc = toml.parse::<Document>()?;
        self.update_toml_dep_table(&mut doc, "dependencies")?;
        self.update_toml_dep_table(&mut doc, "build-dependencies")?;
        self.update_toml_dep_table(&mut doc, "dev-dependencies")?;
        if let Some(workspace) = &doc["workspace"].as_table() {
            if let Some(members) = workspace.get("members") {
                if let Some(members) = members.as_array() {
                    for dirname in members.iter() {
                        if let Some(dirname) = dirname.as_str() {
                            let mut sub_cargo_toml = toml_filename.to_path_buf();
                            sub_cargo_toml.pop();
                            sub_cargo_toml.push(dirname);
                            sub_cargo_toml.push("Cargo.toml");
                            self.clone().update_toml_file(&sub_cargo_toml)?;
                        }
                    }
                }
            }
        }
        Ok(doc.to_string())
    }

    pub fn update_toml_file(&self, toml_filename: &Path) -> Result<(), Box<dyn error::Error>> {
        println!("Reading file: {}", toml_filename.display());

        let mut contents = String::new();
        {
            let mut f = File::open(toml_filename)?;
            f.read_to_string(&mut contents)
                .expect("Something went wrong reading the file.");
        }

        let new_contents = self.update_toml(&contents, toml_filename)?;
        if new_contents == contents {
            return Ok(());
        }

        if !self.unsafe_file_updates {
            let mut toml_filename_old = toml_filename.to_path_buf();
            toml_filename_old.set_file_name(toml_filename_old.file_name()
                                            .expect("No file name specified")
                                            .to_string_lossy().to_string() + ".old");
            let _ = fs::remove_file(&toml_filename_old);
            fs::copy(toml_filename, toml_filename_old)?;
        }

        fs::write(toml_filename, new_contents)?;
        Ok(())
    }

    fn lookup_latest_version(&self, crate_name: &str) -> Result<String, Box<dyn error::Error>> {

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

        let mut version = None;
        let json_doc: Value = serde_json::from_str(&http_body)?;
        if let Some(json_versions) = json_doc["versions"].as_array() {
            for json_version in json_versions {
                if let Some(yanked) = json_version["yanked"].as_bool() {
                    if yanked && !self.allow_yanked {
                        // Skip this yanked version.
                        continue;
                    }
                }
                match json_version["num"].as_str() {
                    Some(version_num) => {
                        match Version::parse(version_num) {
                            Ok(version_num_sem) => {
                                if !version_num_sem.pre.is_empty() && !self.allow_prerelease {
                                    // Skip this pre-release.
                                    continue;
                                }
                                // Found the latest usable version.
                                version = Some(version_num_sem.to_string());
                                // Stop the search here.
                                break;
                            },
                            Err(e) => {
                                return Err(Box::new(Error(format!(
                                    "Crates.io json info for '{}' did not include a valid version 'num': {}",
                                    crate_name, e))));
                            },
                        }
                    },
                    None => {
                        return Err(Box::new(Error(format!(
                            "Crates.io json info for '{}' did not include version 'num'",
                            crate_name))));
                    },
                }
            }
        }

        match version {
            Some(version) => Ok(version),
            None => {
                Err(Box::new(Error(format!(
                    "No usable version found for '{}' on crates.io.",
                    crate_name))))
            }
        }
    }
}
