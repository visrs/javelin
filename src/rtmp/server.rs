use {
    std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    },
    log::{info, error},
    futures::try_ready,
    tokio::{
        prelude::*,
        net::{TcpListener, TcpStream, tcp::Incoming}
    },
};
#[cfg(feature = "tls")]
use {
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
    config: Config,
    shared: Shared,
    listener: Incoming,
    client_id: AtomicUsize,
}

impl Server {
    pub fn new(config: Config, shared: Shared) -> Self {
        let listener = TcpListener::bind(&config.addr).expect("Failed to bind TCP listener");

        info!("Starting up Javelin RTMP server on {}", config.addr);

        Self {
            config,
            shared,
            listener: listener.incoming(),
            client_id: AtomicUsize::default(),
        }
    }

    fn client_id(&self) -> ClientId {
        self.client_id.load(Ordering::SeqCst)
    }

    fn increment_client_id(&mut self) {
        self.client_id.fetch_add(1, Ordering::SeqCst);
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Some(tcp_stream) = try_ready!(self.listener.poll().map_err(|err| error!("{}", err))) {
            spawner(self.client_id(), tcp_stream, self.config.clone(), self.shared.clone());
            self.increment_client_id();
        }

        Ok(Async::Ready(()))
    }
}

fn process<S>(id: ClientId, stream: S, config: Config, shared: Shared)
    where S: AsyncRead + AsyncWrite + Send + 'static
{
    info!("New client connection: {}", id);

    let bytes_stream = BytesStream::new(stream);
    let peer = Peer::new(id, bytes_stream, config, shared)
        .map_err(|err| error!("{}", err));

    tokio::spawn(peer);
}

#[cfg(not(feature = "tls"))]
fn spawner(id: ClientId, stream: TcpStream, config: Config, shared: Shared) {
    stream.set_keepalive(Some(Duration::from_secs(30)))
        .expect("Failed to set TCP keepalive");

    process(id, stream, config, &shared);
}

#[cfg(feature = "tls")]
fn spawner(id: ClientId, stream: TcpStream, config: Config, shared: Shared) {
    stream.set_keepalive(Some(Duration::from_secs(30)))
        .expect("Failed to set TCP keepalive");

    if config.tls.enabled {
        let tls_acceptor = {
            let p12 = config.tls.read_cert().expect("Failed to read TLS certificate");
            let password = &config.tls.cert_password;
            let cert = native_tls::Identity::from_pkcs12(&p12, password).unwrap();
            TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build().unwrap())
        };

        let tls_accept = tls_acceptor.accept(stream)
            .and_then(move |tls_stream| {
                process(id, tls_stream, config, shared);
                Ok(())
            })
            .map_err(|err| {
                error!("TLS error: {:?}", err);
            });

        tokio::spawn(tls_accept);
    } else {
        process(id, stream, config, shared);
    }
}
