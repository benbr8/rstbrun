use clap::{App, Arg};
use fancy_regex::Regex;
use std::path::PathBuf;

fn main() {
    let cla = App::new("Rstb test runner")
        .version("0.1.0")
        .author("benbr8")
        .about("Executes Rstb tests and aggregates results.")
        .arg(Arg::with_name("target")
                .value_name("FOLDER")
                .help("Path(s) to test(s)")
                .required(true)
                .multiple(true)
                .takes_value(true)
        ).arg(Arg::with_name("build")
                .long("build")
                .value_name("FOLDER")
                .help("Folder where cargo build products are placed")
                .takes_value(true)
        ).get_matches();

    let current_dir = std::env::current_dir().expect("Could not get working directory path.");
    let build_dir = match cla.value_of("build") {
        Some(rel_path) => current_dir
            .join(rel_path)
            .canonicalize()
            .expect("Argument 'build' is not a valid path."),
        None => current_dir.join(".rstb_build"),
    };
    let mut target_paths = Vec::new();
    for val in cla.values_of("target").unwrap() {
        let path = current_dir.join(val);
        if !path.is_dir() {
            println!(
                "WARNING: {} is not a valid directory. Skipping.",
                path.to_str().unwrap()
            );
        } else {
            target_paths.push(path.canonicalize().unwrap());
        }
    }
    // dbg!(target_paths);
    let mut test_paths = Vec::new();
    for target_path in target_paths {
        find_tests_in_path(target_path, &mut test_paths);
    }
    // dbg!(test_paths);

    // Compile tests
    let re_crate_name = Regex::new(r#"name = "(\S*)""#).unwrap();
    let mut test_names = Vec::new();
    for test_path in test_paths {
        let cargo_toml = std::fs::read_to_string(test_path.join("Cargo.toml"))
            .expect("Could not read Cargo.toml.");
        let name_captures = re_crate_name.captures(&cargo_toml).unwrap().expect("Could not find test name in Cargo.toml.");
        let name_match = name_captures.get(1).expect("Could not find test name in Cargo.toml.");
        test_names.push(name_match.as_str().to_string());

        let mut cmd = std::process::Command::new("cargo")
            .env("CARGO_TARGET_DIR", build_dir.as_os_str())
            .current_dir(test_path)
            .stdout(std::process::Stdio::inherit())
            .args(vec!["build", "--release"])
            .spawn().unwrap();
        cmd.wait().unwrap();
    }

    // Run tests

}

fn find_tests_in_path(target_path: PathBuf, test_paths: &mut Vec<PathBuf>) {
    let name = target_path.file_name().unwrap().to_string_lossy();
    if name.starts_with("test_") || name.ends_with("_test") {
        // TODO: add some more checks
        if target_path.join("Cargo.toml").is_file() {
            test_paths.push(target_path);
        }
    } else {
        for entry in target_path.read_dir().unwrap().map(|p| p.unwrap()) {
            if entry.path().is_dir() {
                find_tests_in_path(entry.path(), test_paths);
            }
        }
    }
}
