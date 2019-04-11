# dependency-refresh

dependency-refresh is meant to update rust dependency versions within Cargo.toml files.

The tool reads the given toml files and checks online at https://crates.io for the latest version of each dependency.

I am aware that there is room for improvement in my rust code, so feel free to comment or submit small patches.

## Example usage

```sh
$ ./dr /home/jm/IdeaProjects/dependency-refresh/Cargo.toml
Reading file: /home/jm/IdeaProjects/dependency-refresh/Cargo.toml
        Found: structopt
                Local version:  0.2
                Online version: 0.2.15
        Found: toml_edit
                Local version:  0.1.3
                Online version: 0.1.3
        Found: reqwest
                Local version:  0.9.13
                Online version: 0.9.13
        Found: serde_json
                Local version:  1.0
                Online version: 1.0.39
$
```

## Installation

|  Arch linux | https://aur.archlinux.org/packages/rust-dependency-refresh/ |
|-------------|-------------------------------------------------------------|
