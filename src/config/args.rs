use std::{
    collections::{HashMap, HashSet},
};

use config::{Source, Value, ConfigError};
use clap::{
    Arg, App, ArgMatches, AppSettings,
    crate_name,
    crate_version,
    crate_authors,
    crate_description,
};


#[derive(Debug, Clone)]
pub struct Arguments {
    args: ArgMatches<'static>,
}

impl Arguments {
    const SOURCE_URI: &'static str = "arguments";
    const DELIMITER: &'static str = "::";

    pub fn new() -> Self {
        let args = build_args();
        Self { args }
    }

    // TODO: allow XDG based directories?
    pub fn config_dir(&self) -> String {
        self.args.value_of("config_dir")
            .unwrap_or("./config")
            .into()
    }

    fn rtmp_settings(&self) -> Value {
        let mut tmp: HashMap<String, Value> = HashMap::new();

        let host = self.args.value_of("rtmp::bind")
            .expect("BUG: default value for 'bind' missing");
        let port = self.args.value_of("rtmp::port")
            .expect("BUG: default value for 'port' missing");
        let addr = format!("{}:{}", host, port);
        tmp.insert("addr".to_string(), addr.into());

        let republish_action = self.args
            .value_of("rtmp::republish_action")
            .expect("BUG: default value for 'republish_action' missing")
            .parse::<String>()
            .unwrap(); // this should be safe to unwrap
        tmp.insert("republish_action".to_string(), republish_action.into());

        let permitted_stream_keys: Vec<String> = self.args
            .values_of("rtmp::permitted_stream_keys")
            .unwrap_or_default()
            .map(str::to_string)
            .collect::<HashSet<String>>()
            .iter().map(ToOwned::to_owned).collect();
        let value = Value::new(None, permitted_stream_keys);
        tmp.insert("permitted_stream_keys".to_string(), value);

        tmp.into()
    }

    #[cfg(feature = "hls")]
    fn hls_settings(&self) -> Value {
        let mut tmp: HashMap<String, Value> = HashMap::new();

        let enabled = !self.args.is_present("hls::disabled");
        tmp.insert("enabled".into(), enabled.into());

        let root_dir = self.args
            .value_of("hls::root")
            .unwrap_or("./tmp/stream");
        tmp.insert("root_dir".into(), root_dir.into());

        tmp.into()
    }

    #[cfg(feature = "web")]
    fn web_settings(&self) -> Value {
        let mut tmp: HashMap<String, Value> = HashMap::new();

        let enabled = !self.args.is_present("http::disabled");
        tmp.insert("disabled".into(), enabled.into());

        let host = self.args
            .value_of("http::bind")
            .expect("BUG: default value for 'http::bind' missing");
        let port = self.args
            .value_of("http::port")
            .expect("BUG: default value for 'http::port' missing");
        let addr = format!("{}:{}", host, port);
        tmp.insert("addr".into(), addr.into());

        tmp.into()
    }

    #[cfg(feature = "tls")]
    fn tls_settings(&self) -> Value {
        let mut tmp: HashMap<String, Value> = HashMap::new();

        let enabled = self.args.is_present("tls::enabled");
        tmp.insert("enabled".into(), enabled.into());

        let cert_path = self.args.value_of("tls::cert").unwrap_or_default();
        tmp.insert("cert_path".into(), cert_path.into());

        let cert_password = self.args.value_of("tls::password").unwrap_or_default();
        tmp.insert("cert_password".into(), cert_password.into());

        tmp.into()
    }
}

impl Source for Arguments {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut tmp = HashMap::new();

        tmp.insert("config_dir".into(), self.config_dir().into());
        tmp.insert("rtmp".into(), self.rtmp_settings());

        #[cfg(feature = "hls")]
        tmp.insert("hls".into(), self.hls_settings());

        #[cfg(feature = "web")]
        tmp.insert("web".into(), self.web_settings());

        #[cfg(feature = "tls")]
        tmp.insert("tls".into(), self.tls_settings());

        Ok(tmp)
    }
}

fn build_args<'a>() -> ArgMatches<'a> {
    let mut app = App::new(capitalize(crate_name!()))
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(Arg::with_name("rtmp::bind")
            .short("b")
            .long("rtmp-bind")
            .alias("bind")
            .value_name("ADDRESS")
            .default_value("0.0.0.0")
            .help("Host address to bind to"))
        .arg(Arg::with_name("rtmp::port")
            .short("p")
            .long("rtmp-port")
            .alias("port")
            .value_name("PORT")
            .default_value("1935")
            .help("Port to listen on"))
        .arg(Arg::with_name("rtmp::permitted_stream_keys")
            .short("k")
            .long("rtmp-permit-stream-key")
            .alias("permit-stream-key")
            .value_name("KEY")
            .help("Permit a stream key for publishing")
            .multiple(true))
        .arg(Arg::with_name("rtmp::republish_action")
            .long("rtmp-republish-action")
            .alias("republish-action")
            .possible_values(&["replace", "deny"])
            .default_value("replace")
            .help("The action to take when a republishing to the same application"))
        .arg(Arg::with_name("config_dir")
            .short("c")
            .long("config-dir")
            .value_name("PATH")
            .help("The directory where all config files are located"));

    let mut args = Vec::new();

    if cfg!(feature = "web") {
        args.push(Arg::with_name("http::disabled")
            .long("disable-http")
            .help("Disables the integrated web server"));

        args.push(Arg::with_name("http::bind")
            .long("http-bind")
            .value_name("ADDRESS")
            .default_value("0.0.0.0")
            .help("The web server address"));

        args.push(Arg::with_name("http::port")
            .long("http-port")
            .value_name("PORT")
            .default_value("8080")
            .help("The web server listening port"));
    }

    if cfg!(feature = "hls") {
        args.push(Arg::with_name("hls::disabled")
            .long("disable-hls")
            .help("Disables HLS support"));

        args.push(Arg::with_name("hls::root")
            .long("hls-root")
            .value_name("PATH")
            .display_order(20)
            .help("The directory where stream output will be placed"));
    }

    if cfg!(feature = "tls") {
        args.push(Arg::with_name("tls::enabled")
            .long("enable-tls")
            .requires("tls_cert")
            .help("Enables TLS support"));

        args.push(Arg::with_name("tls::cert")
            .long("tls-cert")
            .value_name("CERTIFICATE")
            .help("The TLS certificate to use"));
    }

    app = app.args(&args);

    app.get_matches()
}

fn capitalize(string: &str) -> String {
    string
        .chars()
        .enumerate()
        .map(|(i, c)| {
            match i {
                0 => c.to_uppercase().to_string(),
                _ => c.to_string(),
            }
        })
        .collect()
}
