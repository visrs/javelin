use clap::{
    Arg, App, ArgMatches, AppSettings,
    crate_name,
    crate_version,
    crate_authors,
    crate_description,
};


pub fn build_args<'a>() -> ArgMatches<'a> {
    let mut app = App::new(capitalize(crate_name!()))
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::with_name("config_dir")
            .short("c")
            .long("config-dir")
            .value_name("PATH")
            .help("The directory where all config files are located"));

    let mut args = Vec::new();

    // RTMP
    args.push(Arg::with_name("rtmp_bind")
        .short("b")
        .long("rtmp-bind")
        .alias("bind")
        .value_name("ADDRESS")
        .default_value("0.0.0.0")
        .display_order(1)
        .help("Host address to bind to"));

    args.push(Arg::with_name("rtmp_port")
        .short("p")
        .long("rtmp-port")
        .alias("port")
        .value_name("PORT")
        .default_value("1935")
        .display_order(1)
        .help("Port to listen on"));

    args.push(Arg::with_name("rtmp_permitted_streams")
        .short("k")
        .long("rtmp-permit-stream")
        .alias("permit-stream-key")
        .value_name("KEY")
        .display_order(1)
        .multiple(true)
        .help("Permit a stream key for publishing"));

    args.push(Arg::with_name("rtmp_republish_action")
        .long("rtmp-republish-action")
        .alias("republish-action")
        .possible_values(&["replace", "deny"])
        .default_value("replace")
        .display_order(1)
        .help("The action to take when a republishing to the same application"));

    // RTMPS
    if cfg!(feature = "tls") {
        args.push(Arg::with_name("rtmps_bind")
            .long("rtmps-bind")
            .value_name("ADDRESS")
            .default_value("0.0.0.0")
            .display_order(1)
            .help("Host address to bind to"));

        args.push(Arg::with_name("rtmps_port")
            .long("rtmps-port")
            .value_name("PORT")
            .default_value("1936")
            .display_order(2)
            .help("Port to listen on for RTMPS"));

        args.push(Arg::with_name("rtmps_enabled")
            .long("enable-rtmps")
            .alias("enable-tls")
            .requires("rtmps_tls_cert")
            .display_order(2)
            .help("Enables TLS support"));

        args.push(Arg::with_name("rtmps_tls_cert")
            .long("rtmps-tls-cert")
            .alias("tls-cert")
            .value_name("CERTIFICATE")
            .display_order(2)
            .help("The TLS certificate to use"));
    }

    if cfg!(feature = "web") {
        args.push(Arg::with_name("http_disabled")
            .long("disable-http")
            .help("Disables the integrated web server"));

        args.push(Arg::with_name("http_bind")
            .long("http-bind")
            .value_name("ADDRESS")
            .default_value("0.0.0.0")
            .display_order(10)
            .help("The web server address"));

        args.push(Arg::with_name("http_port")
            .long("http-port")
            .value_name("PORT")
            .default_value("8080")
            .display_order(10)
            .help("The web server listening port"));
    }

    if cfg!(feature = "hls") {
        args.push(Arg::with_name("hls_disabled")
            .long("disable-hls")
            .help("Disables HLS support"));

        args.push(Arg::with_name("hls_root")
            .long("hls-root")
            .value_name("PATH")
            .display_order(20)
            .help("The directory where stream output will be placed"));
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
