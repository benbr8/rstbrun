mod config;

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
            .takes_value(true))
        .arg(Arg::with_name("rstb-build")
            .long("rstb-build")
            .value_name("FOLDER")
            .help("Folder where cargo build products are placed")
            .takes_value(true))
        .arg(Arg::with_name("sim-build")
            .long("sim-build")
            .value_name("FOLDER")
            .help("Folder where cargo build products are placed")
            .takes_value(true)
        ).get_matches();

    let current_dir = std::env::current_dir().expect("Could not get working directory path.");

    let rstb_build_dir = match cla.value_of("rstb-build") {
        Some(rel_path) => current_dir
            .join(rel_path)
            .canonicalize()
            .expect("Argument 'rstb-build' is not a valid path."),
        None => current_dir.join(".rstb_build"),
    };

    let sim_build_dir = match cla.value_of("sim-build") {
        Some(rel_path) => current_dir
            .join(rel_path)
            .canonicalize()
            .expect("Argument 'sim-build' is not a valid path."),
        None => current_dir.join(".sim_build"),
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

    // find test paths
    let mut test_paths = Vec::new();
    for target_path in target_paths {
        find_tests_in_path(target_path, &mut test_paths);
    }
    dbg!(&test_paths);

    // retrieve test names
    let test_names = get_test_names(&test_paths);
    dbg!(&test_names);

    // parse rstb.toml
    let mut test_configs = Vec::new();
    for test_path in &test_paths {
        let config = config::parse_rstb_toml(&test_path.join("rstb.toml"));
        test_configs.push(config);
    }

    // compile tests
    for test_path in &test_paths {
        let mut proc = std::process::Command::new("cargo")
            .env("CARGO_TARGET_DIR", &rstb_build_dir.as_os_str())
            .current_dir(test_path)
            .stdout(std::process::Stdio::inherit())
            .args(vec!["build", "--release"])
            .spawn().unwrap();
        proc.wait().unwrap();
    }

    // Compile sources
    let _ = std::fs::remove_dir_all(&sim_build_dir);
    for j in 0..test_paths.len() {
        let sim_dir = sim_build_dir.join(&test_names[j]);
        let config = &test_configs[j];
        std::fs::create_dir_all(&sim_dir).unwrap();
        let mut args: Vec<String> = Vec::new();
        let out_file = sim_dir.join("sim.vvp").into_os_string().into_string().unwrap();
        args.append(&mut vec![
            "-o".to_string(),
            out_file,
            "-s".to_string(),
            config.test.toplevel.clone(),
            "-g2012".to_string(),
        ]);

        let hdl_files = &config.src.verilog.clone().unwrap();
        for f in hdl_files {
            let s = test_paths[j].join(f).into_os_string().into_string().unwrap();
            args.push(s);
        }
        let mut proc = std::process::Command::new("iverilog")
            .current_dir(sim_dir.clone())
            .stdout(std::process::Stdio::inherit())
            .args(&args)
            .spawn().unwrap();
        proc.wait().unwrap();
    }

    // Run tests
    for name in &test_names {
        println!("\n\n\n");
        println!("###################################################################");
        println!("# RUNNING TEST: {}", name);
        println!("###################################################################\n");
        // rename libs for iverilog
        let mut lib_name = "lib".to_string();
        lib_name.push_str(name);
        let mut lib_name_iverilog = lib_name.clone();
        lib_name_iverilog.push_str(".vpi");
        let mut lib_name_so = lib_name.clone();
        lib_name_so.push_str(".so");

        let sim_dir = sim_build_dir.join(name);
        let lib_path_iverilog = rstb_build_dir.join("release").join(&lib_name_iverilog);
        let lib_path_so = rstb_build_dir.join("release").join(&lib_name_so);
        let _ = std::fs::remove_file(&lib_path_iverilog);
        std::fs::copy(lib_path_so, &lib_path_iverilog).unwrap();

        // run tests
        let rstb_build_dir_string = rstb_build_dir.join("release").into_os_string().into_string().unwrap();
        let test_bin = sim_dir.join("sim.vvp").into_os_string().into_string().unwrap();
        let mut proc = std::process::Command::new("vvp")
            .current_dir(sim_build_dir.join(name))
            .stdout(std::process::Stdio::inherit())
            .args(vec![
                "-M".to_string(),
                rstb_build_dir_string,
                "-m".to_string(),
                lib_name,
                test_bin,
            ])
            .spawn().unwrap();
        proc.wait().unwrap();
    }
}

fn get_test_names(test_paths: &Vec<PathBuf>) -> Vec<String> {
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
