use std::{env, fs, path::PathBuf};

use kelp_config::Config;
use kelp_link::{merge, process_input_file};
use log::info;

fn main() {
    pretty_env_logger::init();

    let mut files = vec![];
    for path in env::args().skip(1) {
        info!("Processing {path:?}");
        let data = fs::read(&path).unwrap();
        files.push((PathBuf::from(path), data));
    }

    let mut inputs = vec![];
    for (path, data) in &files {
        inputs.push(process_input_file(PathBuf::from(path), &data));
    }

    let cfg = Config::default();
    merge(inputs, cfg);
}
