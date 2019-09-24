use {
    std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    },
    log::{info, error, debug},
    futures::try_ready,
    tokio::{
        prelude::*,
        net::{TcpListener, TcpStream, tcp::Incoming}
    },
};
#[cfg(feature = "tls")]
use {
    // TODO: should probably be replaced by rustls
    native_tls,
    tokio_tls::TlsAcceptor,
};
use crate::{
    config::RtmpConfig as Config,
    shared::Shared
};
use super::{
    Peer,
    BytesStream,
    ClientId,
};


pub struct Server {
    client_id: AtomicUsize,
    config: Config,
    shared: Shared,
    rtmp_listener: Incoming,
    #[cfg(feature = "tls")]
    rtmps_listener: Option<Incoming>,
}

impl Server {
    pub fn new(config: Config, shared: Shared) -> Self {
        let rtmp_listener = TcpListener::bind(&config.addr)
            .expect("Failed to bind to RTMP socket")
            .incoming();

        #[cfg(feature = "tls")]
        let rtmps_listener = if config.tls.enabled {
            let listener = TcpListener::bind(&config.tls.addr)
                .expect("Failed to bind to RTMPS socket")
                .incoming();
            Some(listener)
        } else {
            None
        };

        let message = format!("{}", config.addr);

        #[cfg(feature = "tls")]
        let message = if config.tls.enabled {
            format!("{} (RTMP) and {} (RTMPS)", config.addr, config.tls.addr)
        } else {
            message
        };

        info!("Starting up Javelin RTMP server {}", message);

        Self {
            client_id: AtomicUsize::default(),
            config,
            shared,
            rtmp_listener,
            #[cfg(feature = "tls")]
            rtmps_listener,
        }
    }

    fn next_client_id(&self) -> ClientId {
        let id = self.client_id.load(Ordering::SeqCst);
        self.client_id.fetch_add(1, Ordering::SeqCst);
        id
    }

    fn poll_rtmp(&mut self) -> Poll<(), ()> {
        if let Some(tcp_stream) = try_ready!(self.rtmp_listener.poll().map_err(log_error)) {
            tcp_stream
                .set_keepalive(Some(Duration::from_secs(30)))
                .expect("Failed to set TCP keepalive");

            let config = self.config.clone();
            let shared = self.shared.clone();
            let id = self.next_client_id();
            let peer = rtmp_peer(id, tcp_stream, config, shared);

            tokio::spawn(peer);

            return Ok(Async::NotReady);
        }

        Ok(Async::Ready(()))
    }

    #[cfg(feature = "tls")]
    fn poll_rtmps(&mut self) -> Poll<(), ()> {
        if let Some(rtmps_listener) = &mut self.rtmps_listener {
            if let Some(tcp_stream) = try_ready!(rtmps_listener.poll().map_err(log_error)) {
                let id = self.next_client_id();
                let peer = rtmps_peer(id, tcp_stream, self.config.clone(), self.shared.clone());

                tokio::spawn(peer);

                return Ok(Async::NotReady);
            }
        }

        Ok(Async::Ready(()))
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let is_closed = self.poll_rtmp()?.is_ready();

        #[cfg(feature = "tls")]
        let is_closed = self.poll_rtmps()?.is_ready() && is_closed;

        if is_closed {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

fn log_error(e: impl std::error::Error) {
    error!("RTMP: {}", e);
}

fn rtmp_peer<S>(id: ClientId, stream: S, config: Config, shared: Shared) -> impl Future<Item = (), Error = ()>
    where S: AsyncRead + AsyncWrite + Send + 'static
{
    info!("New client connection: {}", id);

    let bytes_stream = BytesStream::new(stream);
    Peer::new(id, bytes_stream, config, shared).map_err(log_error)
}

#[cfg(feature = "tls")]
fn rtmps_peer(id: ClientId, stream: TcpStream, config: Config, shared: Shared) -> impl Future<Item = (), Error = ()> {
    stream
        .set_keepalive(Some(Duration::from_secs(30)))
        .expect("Failed to set TCP keepalive");

    let p12 = config.tls.read_cert().expect("Failed to read TLS certificate");
    let password = &config.tls.cert_password;
    let cert = native_tls::Identity::from_pkcs12(&p12, password).unwrap();

    let tls_acceptor_builder = native_tls::TlsAcceptor::builder(cert).build().unwrap();
    let tls_acceptor = TlsAcceptor::from(tls_acceptor_builder);

    tls_acceptor.accept(stream)
        .map_err(log_error)
        .and_then(move |tls_stream| {
            rtmp_peer(id, tls_stream, config, shared)
        })
}
