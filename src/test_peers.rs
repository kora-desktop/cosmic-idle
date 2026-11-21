// A connected pair of peer-to-peer D-Bus connections for tests.

use std::os::unix::net::UnixStream;

use futures_lite::future;

// The first connection serves `objects`; the second calls them.
pub async fn peers(
    objects: impl FnOnce(
        zbus::connection::Builder<'static>,
    ) -> zbus::Result<zbus::connection::Builder<'static>>,
) -> (zbus::Connection, zbus::Connection) {
    let (server, client) = UnixStream::pair().unwrap();
    let server = objects(
        zbus::connection::Builder::async_io_unix_stream(server)
            .server(zbus::Guid::generate())
            .unwrap()
            .p2p(),
    )
    .unwrap()
    .build();
    let client = zbus::connection::Builder::async_io_unix_stream(client)
        .p2p()
        .build();
    let (server, client) = future::zip(server, client).await;
    (server.unwrap(), client.unwrap())
}
