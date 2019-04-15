pub mod args;
pub mod models;


// use std::{
//     collections::HashSet,
//     path::PathBuf,
// };
// #[cfg(feature = "tls")]
// use std::{
//     fs::File,
//     io::Read,
//     env,
// };

// use log::debug;
// use clap::ArgMatches;

// #[cfg(feature = "tls")]
// use crate::error::Result;

pub use self::models::Settings;


// #[cfg(feature = "tls")]
// impl TlsConfig {
//     pub fn new(args: &ArgMatches) -> Self {
//         let enabled = args.is_present("tls_enabled");

//         if enabled {
//             let cert_path = args.value_of("tls_cert")
//                 .map(|v| Some(PathBuf::from(v)))
//                 .unwrap_or(None);
//             let cert_password = Self::cert_password();
//             Self { cert_path, cert_password, enabled }
//         } else {
//             Self { cert_path: None, cert_password: "".to_string(), enabled }
//         }
//     }

//     fn cert_password() -> String {
//         env::var("JAVELIN_TLS_PASSWORD").expect("Password for TLS certificate required")
//     }

//     pub fn read_cert(&self) -> Result<Vec<u8>> {
//         let path = self.cert_path.clone().expect("");
//         let mut file = File::open(path)?;
//         let mut buf = Vec::with_capacity(2500);
//         file.read_to_end(&mut buf)?;
//         Ok(buf)
//     }
// }

// Loads all stream keys from the configuration file and then from command line arguments.
// Every key is only included once, even if they are specified multiple times.
// fn load_permitted_stream_keys(args: &ArgMatches) -> HashSet<String> {
//     let config_dir = args.value_of("config_dir")
//         .map(PathBuf::from)
//         .unwrap_or_else(|| PathBuf::from("./config"));
//     let keys_file = config_dir.join("permitted_stream_keys.yml");
//     let mut permitted_stream_keys: HashSet<String> = HashSet::new();

//     if keys_file.exists() {
//         debug!("Loading permitted keys from configuration file");
//         if let Ok(file) = std::fs::File::open(&keys_file) {
//             let keys: HashSet<String> = serde_yaml::from_reader(file)
//                 .expect("Failed to read keys from config file");
//             permitted_stream_keys.extend(keys);
//         }
//     }

//     let keys: HashSet<String> = args
//         .values_of("permitted_stream_keys")
//         .unwrap_or_default()
//         .map(str::to_string)
//         .collect();

//     permitted_stream_keys.extend(keys);

//     permitted_stream_keys
// }
