# dependency-refresh

dependency-refresh is meant to update rust dependency versions within Cargo.toml files.

The tool reads the given toml files and checks online at https://crates.io for the latest version of each dependency.

By default dependency-refresh compares the versions according to Semantic versioning (see https://semver.org/) rules the same way Cargo does (see https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html). Therefore, dependency-refresh does not update the version in the local Cargo.toml, if the new crates.io version is a compatible minor update. This behavior usually is desired, because Cargo uses the latest compatible version anyway. To override this, pass the option --exact to dependency-refresh. That will disable Semantic versioning compare and will always trigger an update of the local version.

I am aware that there is room for improvement in my rust code, so feel free to comment or submit small patches.

## Example usage with SemVer (default)

```sh
$ ./target/debug/dr ./Cargo.toml
Reading file: ./Cargo.toml
        Found: structopt
                Local version:  0.3.0
                Online version: 0.3.21  *
        Found: toml_edit
                Local version:  0.2.0
                Online version: 0.2.0
        Found: reqwest
                Local version:  0.11.0
                Online version: 0.11.2  *
        Found: serde_json
                Local version:  1.0.0
                Online version: 1.0.64  *
        Found: semver
                Local version:  0.10.0
                Online version: 0.11.0  *
        Updating: semver 0.10.0 => 0.11.0
```

## Example usage with exact matching (no SemVer)

```sh
$ ./target/debug/dr --exact ./Cargo.toml
Reading file: ./Cargo.toml
        Found: structopt
                Local version:  0.3.0
                Online version: 0.3.21  *
        Found: toml_edit
                Local version:  0.2.0
                Online version: 0.2.0
        Found: reqwest
                Local version:  0.11.0
                Online version: 0.11.2  *
        Found: serde_json
                Local version:  1.0.0
                Online version: 1.0.64  *
        Found: semver
                Local version:  0.10.0
                Online version: 0.11.0  *
        Updating: structopt 0.3.0 => 0.3.21
        Updating: serde_json 1.0.0 => 1.0.64
        Updating: semver 0.10.0 => 0.11.0
        Updating: reqwest 0.11.0 => 0.11.2
```

## Installation

|  Arch linux | https://aur.archlinux.org/packages/rust-dependency-refresh/ |
|-------------|-------------------------------------------------------------|
