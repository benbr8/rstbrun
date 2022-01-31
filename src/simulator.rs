use std::path::PathBuf;
use crate::config::Config;


pub(crate) struct TestEnv {
    pub test_name: String,
    pub test_path: PathBuf,
    pub rstb_build_dir: PathBuf,
    pub sim_build_dir: PathBuf,
    pub force_compile: bool,
    pub config: Config,
}

pub(crate) trait Simulator {
    fn build_test(&self, test: &TestEnv);
    fn run_test(&self, test: &TestEnv);
}


pub struct Icarus ();

impl Simulator for Icarus {
    fn build_test(&self, test: &TestEnv) {
        // build rust
        let mut proc = std::process::Command::new("cargo")
            .env("CARGO_TARGET_DIR", &test.rstb_build_dir.as_os_str())
            .current_dir(&test.test_path)
            .stdout(std::process::Stdio::inherit())
            .args(vec!["build", "--release"])
            .spawn().unwrap();
        proc.wait().unwrap();


        // build hdl
        let sim_dir = test.sim_build_dir.join(&test.test_name);
        let out_file = sim_dir.join("sim.vvp");

        std::fs::create_dir_all(&sim_dir).unwrap();

        let mut args: Vec<String> = Vec::new();
        args.append(&mut vec![
            "-o".to_string(),
            out_file.clone().into_os_string().into_string().unwrap(),
            "-s".to_string(),
            test.config.test.toplevel.clone(),
            "-g2012".to_string(),
        ]);

        let mut hdl_paths = Vec::new();
        for f in &test.config.src.verilog.clone().unwrap() {
            hdl_paths.push(test.test_path.join(f));
        }

        let mut do_compile = test.force_compile;
        if outdated(&[out_file], &hdl_paths) {
            do_compile = true;
        }

        for f in hdl_paths {
            args.push(f.into_os_string().into_string().unwrap());
        }

        if do_compile {
            print!("Running command: iverilog");
            for a in &args {
                print!(" {}", a);
            }
            println!(" ");
            let mut proc = std::process::Command::new("iverilog")
                .current_dir(sim_dir)
                .stdout(std::process::Stdio::inherit())
                .args(&args)
                .spawn().unwrap();
            proc.wait().unwrap();
        }
    }

    fn run_test(&self, test: &TestEnv) {
        let mut lib_name = "lib".to_string();
        lib_name.push_str(&test.test_name);
        let mut lib_name_iverilog = lib_name.clone();
        lib_name_iverilog.push_str(".vpi");
        let mut lib_name_so = lib_name.clone();
        lib_name_so.push_str(".so");

        let sim_dir = test.sim_build_dir.join(&test.test_name);
        let lib_path_iverilog = test.rstb_build_dir.join("release").join(&lib_name_iverilog);
        let lib_path_so = test.rstb_build_dir.join("release").join(&lib_name_so);
        let _ = std::fs::remove_file(&lib_path_iverilog);
        std::fs::copy(&lib_path_so, &lib_path_iverilog).unwrap();

        // run tests
        let rstb_build_dir_string = test.rstb_build_dir.join("release").into_os_string().into_string().unwrap();
        let test_bin = sim_dir.join("sim.vvp").into_os_string().into_string().unwrap();

        let args = vec![
            "-M".to_string(),
            rstb_build_dir_string,
            "-m".to_string(),
            lib_name,
            test_bin,
        ];

        print!("Running command: vvp");
        for a in &args {
            print!(" {}", a);
        }
        println!(" ");
        let mut proc = std::process::Command::new("vvp")
            .current_dir(test.sim_build_dir.join(&test.test_name))
            .stdout(std::process::Stdio::inherit())
            .args(args)
            .spawn().unwrap();
        proc.wait().unwrap();
    }
}

fn outdated(out_files: &[PathBuf], in_files: &[PathBuf]) -> bool {
    let mut newest_in_ts = None;
    for f in in_files {
        let ts = f.metadata().unwrap().modified().unwrap();
        if newest_in_ts.is_none() || (&ts > newest_in_ts.as_ref().unwrap()) {
            newest_in_ts.replace(ts);
        }
    }
    let mut oldest_out_ts = None;
    for f in out_files {
        if let Ok(meta) = f.metadata() {
            let ts = meta.modified().unwrap();
            if oldest_out_ts.is_none() || (&ts < oldest_out_ts.as_ref().unwrap()) {
                oldest_out_ts.replace(ts);
            }
        }
    }

    if let Some(oldest_out_ts) = oldest_out_ts {
        newest_in_ts.unwrap() >= oldest_out_ts
    } else {
        true
    }
}