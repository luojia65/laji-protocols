use std::{
    io,
    net::{ToSocketAddrs, TcpListener, TcpStream, SocketAddr},
};

pub fn listen<A, F, H>(addr: A, factory: F) -> io::Result<()>
where A: ToSocketAddrs, F: FnMut() -> H, H: Handler {
    let mut laji = LajiDiscard::bind(addr, factory)?;
    laji.run()?;
    Ok(()) 
}

pub struct LajiDiscard<F>
where F: Factory {
    tcp: TcpListener,
    factory: F
}

impl<F> LajiDiscard<F>
where F: Factory {
    pub fn bind<A>(addr: A, factory: F) -> io::Result<Self> 
    where A: ToSocketAddrs {
        let tcp = TcpListener::bind(addr)?;
        Ok(LajiDiscard {
            tcp,
            factory
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        for stream in self.tcp.incoming() {
            let mut handler = self.factory.connection_made();
            let stream = stream?;
            handler.on_open(Handshake::read_stream(&stream)?);
            drop(stream);
            handler.on_close();
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Handshake {
    peer_addr: SocketAddr,
    local_addr: SocketAddr,
}

impl Handshake {
    #[inline]
    fn read_stream(ts: &TcpStream) -> io::Result<Self> {
        Ok(Self {
            peer_addr: ts.peer_addr()?,
            local_addr: ts.local_addr()?,
        })
    }

    #[inline]
    pub fn peer_addr(&self) -> &SocketAddr {
        &self.peer_addr
    }

    #[inline]
    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }
}

pub trait Handler {
    fn on_open(&mut self, _shake: Handshake) {}

    fn on_close(&mut self) {}
}

impl<F> Handler for F 
where F: FnMut(Handshake) {
    fn on_open(&mut self, shake: Handshake) {
        self(shake)
    }
}

impl<F1, F2> Handler for (F1, F2) 
where F1: FnMut(Handshake), F2: FnMut() {
    fn on_open(&mut self, shake: Handshake) {
        self.0(shake)
    }

    fn on_close(&mut self) {
        self.1()
    }
}

pub trait Factory {
    type Handler: Handler; 

    fn connection_made(&mut self) -> Self::Handler; 
}

impl<F, H> Factory for F 
where H: Handler, F: FnMut() -> H {
    type Handler = H;

    fn connection_made(&mut self) -> H {
        self()
    }
}

#[cfg(test)]
mod tests {
    mod laji_echo {
        pub use super::super::*;
    }
    use std::thread;
    use std::net::TcpStream;
    #[test]
    fn test_listen() {
        thread::spawn(move || {
            laji_echo::listen("0.0.0.0:9", move || {
                |shake| {
                    println!("Rejected: {:?}", shake);
                }
            }).unwrap();
        });
        thread::spawn(move || {
            laji_echo::listen("0.0.0.0:9999", move || {
                (|shake| {
                    println!("Opened a session: {:?}", shake);
                },
                ||{
                    println!("Closed a session")
                }
                )
            }).unwrap();
        });
        TcpStream::connect("127.0.0.1:9").unwrap();
        TcpStream::connect("127.0.0.1:9999").unwrap();
    }
}
