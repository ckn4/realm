mod zio;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "tfo")] {
        mod tfo;
        use tfo::TcpStream;
        pub use tfo::TcpListener;
    } else {
        use tokio::net::TcpStream;
        pub use tokio::net::TcpListener;
    }
}

use std::io::Result;
use std::net::SocketAddr;
use std::time::Duration;

use log::{warn, debug};

use tokio::net::TcpSocket;

use crate::utils::{RemoteAddr, ConnectOpts};

macro_rules! setsockopt_warn {
    ($op: expr, $opt: expr) => {{
        let _ = $op.map_err(|e| warn!("[tcp]failed to setsockopt $opt: {}", e));
    }};
}

#[allow(unused_variables)]
pub async fn proxy(
    mut inbound: TcpStream,
    remote: RemoteAddr,
    conn_opts: ConnectOpts,
) -> Result<(u64, u64)> {
    let ConnectOpts {
        tcp_timeout: timeout,
        fast_open,
        zero_copy,
        send_through,
        ..
    } = conn_opts;

    let remote = remote.into_sockaddr().await?;

    debug!("[tcp]remote resolved as {}", &remote);

    let timeout = if timeout != 0 {
        Some(Duration::from_secs(timeout))
    } else {
        None
    };

    let mut outbound = match send_through {
        Some(x) => {
            let socket = match x {
                SocketAddr::V4(_) => TcpSocket::new_v4()?,
                SocketAddr::V6(_) => TcpSocket::new_v6()?,
            };

            setsockopt_warn!(socket.set_reuseaddr(true), "reuseaddr");

            #[cfg(unix)]
            setsockopt_warn!(socket.set_reuseport(true), "reuseport");

            socket.bind(x)?;

            #[cfg(feature = "tfo")]
            if fast_open {
                TcpStream::connect_with_socket(socket, remote).await?
            } else {
                socket.connect(remote).await?.into()
            }

            #[cfg(not(feature = "tfo"))]
            socket.connect(remote).await?
        }
        None => TcpStream::connect(remote).await?,
    };

    setsockopt_warn!(inbound.set_nodelay(true), "nodelay");
    setsockopt_warn!(outbound.set_nodelay(true), "nodelay");

    #[cfg(all(target_os = "linux", feature = "zero-copy"))]
    let res = if zero_copy {
        zio::bidi_copy_pipe(&mut inbound, &mut outbound).await
    } else {
        zio::bidi_copy_buffer(&mut inbound, &mut outbound).await
    };

    #[cfg(not(all(target_os = "linux", feature = "zero-copy")))]
    let res = zio::bidi_copy_buffer(&mut inbound, &mut outbound).await;

    res
}