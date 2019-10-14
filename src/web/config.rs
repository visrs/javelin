use {
    std::net::SocketAddr,
    clap::ArgMatches,
};


#[derive(Debug, Clone)]
#[cfg(feature = "web")]
pub struct Config {
    pub addr: SocketAddr,
    pub enabled: bool,
}

#[cfg(feature = "web")]
impl Config {
    pub fn new(args: &ArgMatches) -> Self {
        let enabled = !args.is_present("http_disabled");

        let host = args.value_of("http_bind").expect("BUG: default value for 'http_bind' missing");
        let port = args.value_of("http_port").expect("BUG: default value for 'http_port' missing");
        let addr = format!("{}:{}", host, port).parse().expect("Invalid address or port name for web server");

        Self { addr, enabled }
    }
}
