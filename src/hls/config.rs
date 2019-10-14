use {
    std::path::PathBuf,
    clap::ArgMatches,
};


#[derive(Debug, Clone)]
#[cfg(feature = "hls")]
pub struct Config {
    pub root_dir: PathBuf,
    pub enabled: bool,
}

#[cfg(feature = "hls")]
impl Config {
    pub fn new(args: &ArgMatches) -> Self {
        let enabled = !args.is_present("hls_disabled");

        let root_dir = args.value_of("hls_root")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./tmp/stream"));

        Self { root_dir, enabled }
    }
}



