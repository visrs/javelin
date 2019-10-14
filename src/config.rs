use {
    std::{
        io,
        collections::HashMap,
        net::SocketAddr,
        str::FromStr,
        result,
        path::PathBuf,
    },
    snafu::{Snafu, ResultExt},
    log::{debug, error},
    clap::ArgMatches,
};
#[cfg(feature = "tls")]
use std::{
    fs::File,
    io::Read,
    env,
};
use crate::{
    args,
    web::Config as WebConfig,
    hls::Config as HlsConfig,
};


#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed to parse {}: {}", what, message))]
    ParseError { what: String, message: String },

    #[cfg(feature = "tls")]
    #[snafu(display("Could not find TLS certificate at {}", path.display()))]
    NoCertificateFound { source: io::Error, path: PathBuf },

    #[cfg(feature = "tls")]
    #[snafu(display("Failed to read file {}: {}", path.display(), source))]
    ReadError { source: io::Error, path: PathBuf }
}

type Result<T, E = Error> = std::result::Result<T, E>;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepublishAction {
    Replace,
    Deny,
}

impl FromStr for RepublishAction {
    type Err = Error;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let action = match s {
            "replace" => RepublishAction::Replace,
            "deny" => RepublishAction::Deny,
            _ => {
                return Err(Error::ParseError {
                    what: "RepublishAction".into(),
                    message: format!("'{}' not valid", s)
                });
            }
        };

        Ok(action)
    }
}


#[derive(Debug, Clone)]
#[cfg(feature = "tls")]
pub struct TlsConfig {
    pub addr: SocketAddr,
    pub cert_path: Option<PathBuf>,
    pub cert_password: String,
    pub enabled: bool,
}

#[cfg(feature = "tls")]
impl TlsConfig {
    pub fn new(args: &ArgMatches) -> Self {
        let enabled = args.is_present("rtmps_enabled");
        let port = args.value_of("rtmps_port").expect("Failed to get value");
        let bind = args.value_of("rtmps_bind").expect("Failed to get value");
        let addr = format!("{}:{}", bind, port).parse::<SocketAddr>().unwrap();

        if enabled {
            let cert_path = args.value_of("rtmps_tls_cert")
                .map(|v| Some(PathBuf::from(v)))
                .unwrap_or(None);
            let cert_password = Self::cert_password();
            Self { addr, cert_path, cert_password, enabled }
        } else {
            Self { addr, cert_path: None, cert_password: "".to_string(), enabled }
        }
    }

    fn cert_password() -> String {
        env::var("JAVELIN_TLS_PASSWORD").expect("Password for TLS certificate required")
    }

    pub fn read_cert(&self) -> Result<Vec<u8>> {
        let path = self.cert_path.clone().expect("");
        let mut file = File::open(path.clone()).context(NoCertificateFound { path: path.clone() })?;
        let mut buf = Vec::with_capacity(2500);
        file.read_to_end(&mut buf).context(ReadError { path: path.clone() })?;
        Ok(buf)
    }
}


#[derive(Debug, Clone)]
pub struct RtmpConfig {
    pub addr: SocketAddr,
    pub permitted_stream_keys: KeyRegistry,
    pub republish_action: RepublishAction,
    #[cfg(feature = "tls")]
    pub tls: TlsConfig,
}

impl From<&ArgMatches<'_>> for RtmpConfig {
    fn from(args: &ArgMatches) -> Self {
        let permitted_stream_keys = load_permitted_stream_keys(&args);

        let host = args.value_of("rtmp_bind").expect("BUG: default value for 'rtmp_bind' missing");
        let port = args.value_of("rtmp_port").expect("BUG: default value for 'rtmp_port' missing");
        let addr = format!("{}:{}", host, port).parse().expect("Invalid address or port name");

        let republish_action = args
            .value_of("rtmp_republish_action")
            .expect("BUG: default value for 'republish_action' missing")
            .parse()
            .unwrap(); // this should be safe to unwrap

        Self {
            addr,
            permitted_stream_keys,
            republish_action,
            #[cfg(feature = "tls")]
            tls: TlsConfig::new(&args),
        }
    }
}


type KeyRegistry = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct Config {
    pub rtmp: RtmpConfig,
    #[cfg(feature = "hls")]
    pub hls: HlsConfig,
    #[cfg(feature = "web")]
    pub web: WebConfig,
}

impl Config {
    pub fn new() -> Self {
        let matches = args::build_args();


        Self {
            rtmp: RtmpConfig::from(&matches),
            #[cfg(feature = "hls")]
            hls: HlsConfig::new(&matches),
            #[cfg(feature = "web")]
            web: WebConfig::new(&matches),
        }
    }
}

/// Loads all stream keys from the configuration file and then from command line arguments.
/// Every key is only included once, even if they are specified multiple times.
fn load_permitted_stream_keys(args: &ArgMatches) -> KeyRegistry {
    let config_dir = args.value_of("config_dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./config"));
    let keys_file = config_dir.join("permitted_stream_keys.yml");
    let mut permitted_stream_keys: KeyRegistry = HashMap::new();

    if keys_file.exists() {
        debug!("Loading permitted keys from configuration file");
        if let Ok(file) = std::fs::File::open(&keys_file) {
            let keys: KeyRegistry = serde_yaml::from_reader(file)
                .expect("Failed to read keys from config file");
            permitted_stream_keys.extend(keys);
        }
    }

    let keys = args
        .values_of("rtmp_permitted_streams")
        .unwrap_or_default()
        .fold(HashMap::new(), |mut acc, elem| {
            let tmp = elem.split(':').collect::<Vec<_>>();
            match (tmp.first(), tmp.last()) {
                (Some(app_name), Some(stream_key)) => {
                    acc.insert(app_name.to_string(), stream_key.to_string());
                },
                // TODO: handle this as error
                _ => error!("Invalid stream key provided, skipping")
            }
            acc
        });

    permitted_stream_keys.extend(keys);

    permitted_stream_keys
}
