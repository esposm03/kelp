use std::ffi::CStr;

use indexmap::IndexMap;
use regex::{Regex, bytes::RegexSet};

pub struct Config {
    outputs: Vec<String>,
    regexset: RegexSet,
}

impl Config {
    pub fn parse<R: std::io::Read>(src: R) -> Config {
        let input_glob = Regex::new(r"[a-zA-Z0-9._*-]+").unwrap();

        // A map from output name to a list of input section names that should match
        let deser: IndexMap<String, Vec<String>> = serde_yml::from_reader(src).unwrap();

        let mut outputs = vec![];
        let mut patterns = vec![];
        for (out, pats) in deser {
            for pat in pats {
                outputs.push(out.clone());
                assert!(input_glob.is_match(&pat));
                let regex = format!("^{}$", pat.replace(".", r"\.").replace("*", "[^.]*"));
                patterns.push(regex);
            }
        }

        let regexset = RegexSet::new(patterns).expect("An invalid configuration was given");

        Config { outputs, regexset }
    }

    pub fn output_section<'a>(&'a self, section: &CStr) -> Option<&'a str> {
        self.regexset
            .matches(section.to_bytes())
            .iter()
            .next()
            .map(|i| self.outputs[i].as_str())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::parse(include_bytes!("default_config.yml").as_slice())
    }
}
