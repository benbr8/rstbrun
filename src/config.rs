use std::path::PathBuf;
use serde_derive::Deserialize;


#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub test: Test,
    pub src: Src,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Src {
    pub verilog: Option<Vec<String>>,
    pub vhdl: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Test {
    pub toplevel: String,
}



pub fn parse_rstb_toml(file: &PathBuf) -> Config {
    let cfg_str = std::fs::read_to_string(file).expect("Could not read rstb.toml file.");
    let config: Config = toml::from_str(&cfg_str).expect("Could not parse 'rstb.toml'.");
    // dbg!(&config);
    config
}