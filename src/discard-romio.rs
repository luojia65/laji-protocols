#![feature(async_await, await_macro, futures_api)]
use romio::tcp::{TcpListener, TcpStream};
use futures::prelude::*;

async fn say_hello(mut stream: TcpStream) {
    await!(stream.write_all(b"Shall I hear more, or shall I speak at this?"));
}

async fn listen() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let socket_addr = "127.0.0.1:8080".parse()?;
    let listener = TcpListener::bind(&socket_addr)?;
    let mut incoming = listener.incoming();

    // accept connections and process them serially
    while let Some(stream) = await!(incoming.next()) {
        await!(say_hello(stream?));
    }
    Ok(())
}
