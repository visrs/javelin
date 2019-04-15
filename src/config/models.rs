use std::{
    path::PathBuf,
    net::SocketAddr,
    collections::HashSet,
    // str::FromStr,
    // result::Result as StdResult,
};

use serde_derive::Deserialize;
use config::{
    self,
    Config,
    File as ConfigFile,
    Environment as ConfigEnv,
};

use super::args::Arguments;
use crate::{Error, Result};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum RepublishAction {
    Replace,
    Deny,
}

// impl FromStr for RepublishAction {
//     type Err = Error;

//     fn from_str(s: &str) -> StdResult<Self, Self::Err> {
//         let action = match s {
//             "replace" => RepublishAction::Replace,
//             "deny" => RepublishAction::Deny,
//             _ => return Err(Error::from(format!("Failed to parse RepublishAction, '{}' not valid", s)))
//         };

//         Ok(action)
//     }
// }


#[derive(Debug, Clone, Deserialize)]
#[cfg(feature = "tls")]
pub struct Tls {
    pub cert_path: Option<PathBuf>,
    pub cert_password: String,
    pub enabled: bool,
}


#[derive(Debug, Clone, Deserialize)]
#[cfg(feature = "hls")]
pub struct Hls {
    pub root_dir: PathBuf,
    pub enabled: bool,
}


#[derive(Debug, Clone, Deserialize)]
#[cfg(feature = "web")]
pub struct Web {
    pub addr: SocketAddr,
    pub enabled: bool,
}


#[derive(Debug, Clone, Deserialize)]
pub struct Rtmp {
    pub addr: SocketAddr,
    pub permitted_stream_keys: HashSet<String>,
    pub republish_action: RepublishAction,
}


#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub config_file: PathBuf,
    pub rtmp: Rtmp,
    #[cfg(feature = "tls")]
    pub tls: Tls,
    #[cfg(feature = "hls")]
    pub hls: Hls,
    #[cfg(feature = "web")]
    pub web: Web,
}

impl Settings {
    pub fn new() -> Result<Self> {
        let mut conf = Config::new();

        let arg_conf = Arguments::new();

        let config_file = format!("{}/{}", arg_conf.config_dir(), "settings.yml");
        let file_conf = ConfigFile::with_name(&config_file).required(true);

        let env_conf = ConfigEnv::with_prefix(env!("CARGO_PKG_NAME"));

        conf.merge(file_conf)?;
        println!("{:#?}", conf);
        conf.merge(arg_conf)?;
        println!("{:#?}", conf);
        conf.merge(env_conf)?;
        println!("{:#?}", conf);

        conf.set("config_file", config_file)?;

        Ok(conf.try_into()?)
    }
}
