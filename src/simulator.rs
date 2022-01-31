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
        if outdated(&vec![out_file.clone()], &hdl_paths) {
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
            print!("\n");
            let mut proc = std::process::Command::new("iverilog")
                .current_dir(sim_dir.clone())
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

        let sim_dir = test.sim_build_dir.join(test.test_name);
        let lib_path_iverilog = test.rstb_build_dir.join("release").join(&lib_name_iverilog);
        let lib_path_so = test.rstb_build_dir.join("release").join(&lib_name_so);
        let _ = std::fs::remove_file(&lib_path_iverilog);
        std::fs::copy(&lib_path_so, &lib_path_iverilog).unwrap();
    }
}

fn outdated(out_files: &Vec<PathBuf>, in_files: &Vec<PathBuf>) -> bool {
    let mut newest_in_ts = None;
    for f in in_files {
        let ts = f.metadata().unwrap().modified().unwrap();
        if newest_in_ts.is_none() {
            newest_in_ts.replace(ts);
        } else if &ts > newest_in_ts.as_ref().unwrap() {
            newest_in_ts.replace(ts);
        }
    }
    let mut oldest_out_ts = None;
    for f in out_files {
        let ts = f.metadata().unwrap().modified().unwrap();
        if oldest_out_ts.is_none() {
            oldest_out_ts.replace(ts);
        } else if &ts < oldest_out_ts.as_ref().unwrap() {
            oldest_out_ts.replace(ts);
        }
    }

    newest_in_ts >= oldest_out_ts
}