[![javelin][project_banner]][website_url]

# Javelin RTMP Server

> Please be aware that this software is still under development.
> There could be breaking changes before version 1.0.

Streaming server written in Rust.

Supported sources:
- RTMP

Supported outputs:
- RTMP
- HLS (H.264 + AAC)

## How to install and run

### Via Cargo

```sh
cargo install javelin
# Make sure your $CARGO_HOME/bin is in your $PATH
javelin --permit-stream-key="username:mysecretstreamkey"
```

### Via Docker

```sh
docker pull registry.gitlab.com/valeth/javelin:latest
docker run --tty -p 1935:1935 \
    registry.gitlab.com/valeth/javelin:latest \
    --hls-root=/tmp/streamout \
    --permit-stream-key="username:123456"
```

> Try `javelin --help` for more command line options.


<!-- links -->

[project_banner]: https://files.valeth.info/javelin_banner.png
[website_url]: https://valeth.info
