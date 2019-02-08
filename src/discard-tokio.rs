// todo: encapsule

use tokio::prelude::*;
use tokio::net::TcpListener;

#[allow(unused)]
fn main() {
    // Bind the server's socket.
    let addr = "127.0.0.1:12345".parse().unwrap();
    let listener = TcpListener::bind(&addr)
        .expect("unable to bind TCP listener");

    // Pull out a stream of sockets for incoming connections
    let server = listener.incoming()
        .map_err(|e| eprintln!("accept failed = {:?}", e))
        .for_each(|sock| {
            drop(sock);
            tokio::spawn(future::ok(()))
        });

    // Start the Tokio runtime
    tokio::run(server);
}