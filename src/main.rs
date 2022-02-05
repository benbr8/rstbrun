mod config;
mod simulator;

use clap::{App, Arg};
use fancy_regex::Regex;
use std::{path::PathBuf, str::FromStr, fs};

fn main() {
    let cla = App::new("Rstb test runner")
        .version("0.1.0")
        .author("benbr8")
        .about("Executes Rstb tests and aggregates results.")
        .arg(Arg::with_name("simulator")
            .long("simulator")
            .value_name("SIMULATOR")
            .help("Simulator to run the simulation.")
            .takes_value(true))
        .arg(Arg::with_name("target")
            .value_name("FOLDER")
            .help("Path(s) to test(s). Defaults to current directory.")
            .multiple(true)
            .takes_value(true))
        .arg(Arg::with_name("rstb-dir")
            .long("rstb-dir")
            .value_name("FOLDER")
            .help("Folder where cargo build products are placed")
            .takes_value(true))
        .arg(Arg::with_name("sim-dir")
            .long("sim-dir")
            .value_name("FOLDER")
            .help("Folder where simulator build and output products are placed")
            .takes_value(true))
        .arg(Arg::with_name("compile-only")
            .long("compile-only")
            .help("Compile sources, but do not run tests"))
        .arg(Arg::with_name("force-compile")
            .long("force-compile")
            .help("Force compile HDL sources."))
        .get_matches();

    let force_compile = cla.is_present("force-compile");
    let compile_only = cla.is_present("compile-only");

    let current_dir = std::env::current_dir().expect("Could not get working directory path.");

    let simulator = cla.value_of("simulator").unwrap_or("icarus");
    dbg!(simulator);

    let rstb_dir = match cla.value_of("rstb-dir") {
        Some(rel_path) => current_dir
            .join(rel_path)
            .canonicalize()
            .expect("Argument 'rstb-dir' is not a valid path."),
        None => current_dir.join(".rstb_build"),
    };

    let sim_dir = match cla.value_of("sim-dir") {
        Some(rel_path) => current_dir
            .join(rel_path)
            .canonicalize()
            .expect("Argument 'sim-dir' is not a valid path."),
        None => current_dir.join(".sim_build"),
    };

    let mut target_paths = Vec::new();
    if let Some(values) = cla.values_of("target") {
        for val in values {
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
    } else {
        target_paths.push(current_dir.clone());
    }
    // dbg!(target_paths);

    // find test paths
    let mut test_paths = Vec::new();
    for target_path in target_paths {
        find_tests_in_path(target_path, &mut test_paths);
    }
    // dbg!(&test_paths);

    // retrieve test names
    let test_names = get_test_names(&test_paths);
    // dbg!(&test_names);

    // parse rstb.toml
    let mut test_configs = Vec::new();
    for test_path in &test_paths {
        let mut config = config::parse_rstb_toml(&test_path.join("rstb.toml"));
        normalize_cfg(&mut config);
        test_configs.push(config);
    }

    let mut tests = Vec::new();
    for j in 0..test_paths.len() {
        tests.push(simulator::TestEnv {
            test_name: test_names[j].clone(),
            test_path: test_paths[j].clone(),
            rstb_dir: rstb_dir.clone(),
            sim_dir: sim_dir.clone(),
            force_compile,
            config: test_configs[j].clone(),
        });
    }

    let sim: Box<dyn simulator::Simulator> = match simulator {
        "icarus" => Box::new(simulator::Icarus()),
        _ => panic!()
    };

    for test in &tests {

        println!("# Building test: {}", test.test_name);
        println!("-------------------------------------------------------------------");
        sim.build_test(test);
        if !compile_only {
            println!("\n");
            println!("# Running test: {}", test.test_name);
            println!("-------------------------------------------------------------------");
            sim.run_test(test);
            println!("\n\n");
        }
    }

}

fn get_test_names(test_paths: &[PathBuf]) -> Vec<String> {
    let re_crate_name = Regex::new(r#"name = "(\S*)""#).unwrap();
    let mut test_names = Vec::new();
    for test_path in test_paths {
        let cargo_toml = std::fs::read_to_string(test_path.join("Cargo.toml"))
            .expect("Could not read Cargo.toml.");
        let name_captures = re_crate_name.captures(&cargo_toml).unwrap().expect("Could not find test name in Cargo.toml.");
        let name_match = name_captures.get(1).expect("Could not find test name in Cargo.toml.");
        test_names.push(name_match.as_str().to_string());
    }
    test_names
}

fn find_tests_in_path(target_path: PathBuf, test_paths: &mut Vec<PathBuf>) {
    let name = target_path.file_name().unwrap().to_string_lossy();
    if name.starts_with("test_") || name.ends_with("_test") {
        // TODO: add some more checks
        if target_path.join("Cargo.toml").is_file() && target_path.join("rstb.toml").is_file(){
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

fn normalize_cfg(cfg: &mut config::Config) {
    if let Some(src) = &mut cfg.src.verilog {
        for path in src {
            normalize_path(path);
        }
    }
}

fn normalize_path(path: &mut String) {
    let pb = PathBuf::from_str(path).expect("Given path does not exist.");
    let abs_path = fs::canonicalize(&path).expect("Could not get absolute path.");
    *path = abs_path.into_os_string().into_string().unwrap();
}
