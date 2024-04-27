//! This module contains the actual server code itself

use crate::{QuoteCategory, Quotes};
use anyhow::Context;
#[cfg(feature = "cli")]
use clap::ValueEnum;
use std::sync::Arc;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, ToSocketAddrs, UdpSocket},
    sync::{
        mpsc::{channel, Sender},
        oneshot,
    },
};
use tracing::{debug, error, info, instrument, trace, warn, Instrument};

struct GetQotd(oneshot::Sender<Vec<u8>>);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum AllowedCategories {
    #[default]
    Decorous,
    Offensive,
    All,
}

impl AllowedCategories {
    pub fn as_category_vec(&self) -> Vec<QuoteCategory> {
        match *self {
            AllowedCategories::Decorous => vec![QuoteCategory::Decorous],
            AllowedCategories::Offensive => vec![QuoteCategory::Offensive],
            AllowedCategories::All => vec![QuoteCategory::Decorous, QuoteCategory::Offensive],
        }
    }
}

#[derive(Debug, Default)]
pub struct Server {
    tcp_socket: Option<TcpListener>,
    udp_socket: Option<UdpSocket>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    #[instrument(skip(self))]
    pub async fn bind<A: ToSocketAddrs + std::fmt::Debug>(
        mut self,
        address: A,
    ) -> anyhow::Result<Self> {
        trace!("Binding TCP socket");
        let tcp_socket = TcpListener::bind(address)
            .await
            .context("Failed to bind TCP port")?;
        debug!("Bound to TCP {}", tcp_socket.local_addr()?);

        // If user specifies e.g. port 0, meaning "choose one for me", reading TCP socket's address
        // ensures that we open the same port number for the UDP socket
        trace!("Binding UDP socket");
        self.udp_socket = Some(
            UdpSocket::bind(
                tcp_socket
                    .local_addr()
                    .context("Could not read local address")?,
            )
            .await
            .context("Failed to bind UDP port")?,
        );
        debug!(
            "Bound to UDP {}",
            self.udp_socket.as_ref().unwrap().local_addr()?
        );
        self.tcp_socket = Some(tcp_socket);

        Ok(self)
    }

    /// Drop elevated privileges
    ///
    /// This is currently a no-op on non-Unix/non-Unix-like systems (e.g. Windows)
    #[instrument(skip(self))]
    pub fn drop_privileges(self, name: &str) -> anyhow::Result<Self> {
        #[cfg(unix)]
        {
            use nix::unistd::{setgid, setuid, User};

            if let Some(user) = User::from_name(name).context("Failed to get user")? {
                // Must drop gid first: dropping uid first robs us of our permissions to change our gid!
                setgid(user.gid)
                    .context(format!("Failed to set gid: {}", user.gid))
                    .and_then(|_| {
                        setuid(user.uid).context(format!("Failed to set uid: {}", user.uid))
                    })
                    .unwrap_or_else(|e| {
                        warn!("Failed to drop user privileges: {e:?}");
                    });
            }
        }

        Ok(self)
    }

    #[instrument(skip_all)]
    pub async fn serve(self, mut quotes: Quotes) -> anyhow::Result<()> {
        // Get our bound ports
        let tcp = self.tcp_socket.context("Not bound to TCP socket")?;
        let udp = Arc::new(self.udp_socket.context("Not bound to UDP socket")?);

        let local_addr = tcp.local_addr()?;
        info!(
            "Now listening on TCP/UDP {}:{}",
            local_addr.ip(),
            local_addr.port()
        );

        let (getqotd_tx, mut getqotd_rx) = channel::<GetQotd>(32);

        tokio::spawn(
            async move {
                loop {
                    let quote = quotes
                        .random_quote()
                        .await
                        .context("Failed to choose quote")?;
                    debug!("Chose quote, waiting");
                    if let Some(getter) = getqotd_rx.recv().await {
                        info!("Sending quote to requesting task");
                        let _ = getter.0.send(quote);
                    } else {
                        error!("Quote channel closed!");
                        break Err::<(), _>(anyhow::Error::msg("Quote channel closed"));
                    }
                }
            }
            .instrument(tracing::debug_span!("quote_task")),
        );

        let mut buf = [0_u8; 0];
        loop {
            if getqotd_tx.is_closed() {
                panic!("Quote channel closed!");
            }

            tokio::select! {
                client = tcp.accept() => {
                    let (mut conn, _) = client.context("Failed to connect TCP client")?;
                    info!("TCP client connected: {}", conn.peer_addr()?);
                    let get_tx = getqotd_tx.clone();
                    tokio::spawn(async move {
                        info!("Getting quote");
                        let quote = Self::get_quote(&get_tx).await?;
                        info!("Sending quote to client");
                        conn.write_all(&quote).await?;
                        info!("Done! Closing connection");
                        anyhow::Ok(())
                    }.instrument(tracing::info_span!("tcp_server")));
                },
                client = udp.recv_from(&mut buf) => {
                    let (_, addr) = client.context("Failed to connect UDP client")?;
                    info!("UDP client connected: {}", addr);
                    let get_tx = getqotd_tx.clone();
                    let udp = udp.clone();
                    tokio::spawn(async move {
                        loop {
                            info!("Getting quote");
                            let quote = Self::get_quote(&get_tx).await?;
                            if quote.len() < 512 {
                                info!("Sending quote to client");
                                udp.send_to(&quote, addr).await?;
                                info!("Done! Closing connection");
                                break anyhow::Ok(());
                            }
                            info!("Quote too long for UDP client ({}), retrying", quote.len());
                        }
                    }.instrument(tracing::info_span!("udp_server")));
                },
            };
        }
    }

    async fn get_quote(tx: &Sender<GetQotd>) -> anyhow::Result<Vec<u8>> {
        let (quote_tx, quote_rx) = oneshot::channel();
        tx.send(GetQotd(quote_tx)).await?;
        Ok(quote_rx.await?)
    }
}
