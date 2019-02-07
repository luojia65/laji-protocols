use mio::{Poll, PollOpt, Ready, Token, Events, net::{TcpListener, TcpStream}};
use std::{io, net::{ToSocketAddrs, SocketAddr}};
use slab::Slab;

pub fn listen<A, F, H>(addr: A, factory: F) -> io::Result<()>
where 
    A: ToSocketAddrs, 
    F: FnMut() -> H,
    F: Send + Sync + 'static,
    H: Handler 
{
    Builder::new().bind(addr)?.build(factory)?.run()
}

pub struct LajiDiscard<F> 
where F: Factory 
{
    poll: Poll,
    listeners: Slab<TcpListener>,
    factory: F,
}

impl<F> LajiDiscard<F>
where F: Factory
{
    fn from_tcp(tcp: Vec<TcpListener>, factory: F) -> io::Result<Self> {
        let poll = Poll::new()?;
        let mut listeners = Slab::new();
        for listener in tcp {
            let entry = listeners.vacant_entry();
            let token = Token(entry.key().into());
            poll.register(&listener, token, Ready::readable(), PollOpt::edge())?;
            entry.insert(listener);
        }
        let ans = Self {
            poll,
            listeners,
            factory,
        };
        Ok(ans)
    }
}

impl<F> LajiDiscard<F> 
where F: Factory 
{
    pub fn run(mut self) -> io::Result<()> {
        let mut events = Events::with_capacity(1024);
        loop {
            self.poll.poll(&mut events, None)?;
            for event in &events {
                let token_index = event.token().into();
                if let Some(listener) = self.listeners.get(token_index) {
                    loop {
                        match listener.accept() {
                            Ok((stream, _addr)) => {
                                process_one_stream(&mut self.factory, stream)?;
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }
    }
}

fn process_one_stream<F>(factory: &mut F, stream: TcpStream) -> io::Result<()> 
where F: Factory
{
    let mut handler = factory.connection_made();
    handler.on_open(Handshake::read_stream(&stream)?);
    drop(stream);
    handler.on_close();
    Ok(())
}

#[derive(Debug)]
pub struct Builder {
    tcp: Vec<TcpListener>,
}

impl Builder {
    #[inline]
    pub fn new() -> Self {
        Self { tcp: Vec::new() }
    }

    #[inline]
    pub fn bind<A>(mut self, addr: A) -> io::Result<Builder> 
    where A: ToSocketAddrs 
    {
        let new_listener = TcpListener::from_std(std::net::TcpListener::bind(addr)?)?;
        self.tcp.push(new_listener);
        Ok(self)
    }

    #[inline]
    pub fn build<F>(self, factory: F) -> io::Result<LajiDiscard<F>> 
    where F: Factory
    {
        LajiDiscard::from_tcp(self.tcp, factory)
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
    #[inline]
    fn on_open(&mut self, shake: Handshake) {
        self(shake)
    }

    #[inline]
    fn on_close(&mut self) {}
}

pub trait Factory {
    type Handler: Handler; 

    fn connection_made(&mut self) -> Self::Handler; 
}

impl<F, H> Factory for F 
where 
    H: Handler, 
    F: FnMut() -> H 
{
    type Handler = H;

    #[inline]
    fn connection_made(&mut self) -> H {
        self()
    }
}

#[cfg(test)]
mod tests {
    mod laji_discard {
        pub use super::super::*;
    }
    use std::thread;
    use std::net::TcpStream;

    #[test]
    fn listen() {
        laji_discard::listen("0.0.0.0:9", move || {
            |shake: super::Handshake| {                      
                println!("Remote {} connected to {}", shake.peer_addr(), shake.local_addr());
            } 
        }).unwrap();
    }

    #[test]
    fn test_listen() {
        thread::spawn(move || {
            laji_discard::listen("0.0.0.0:9", move || {
                |shake: super::Handshake| {  
                println!("Remote {} connected to {}", shake.peer_addr(), shake.local_addr());
                } 
            }).unwrap();
        });
        TcpStream::connect("127.0.0.1:9").unwrap();
    }

    #[test]
    fn test_batch() -> std::io::Result<()> {
        use super::*;
        struct MyFactory;
        impl Factory for MyFactory {
            type Handler = MyHandler;
            fn connection_made(&mut self) -> MyHandler {
                MyHandler(None)
            }
        }
        struct MyHandler(Option<Handshake>);
        impl Handler for MyHandler {
            fn on_open(&mut self, shake: Handshake) {                
                println!("Remote {} connected to {}", shake.peer_addr(), shake.local_addr());
                self.0 = Some(shake);
            }
            fn on_close(&mut self) {
                let shake = self.0.unwrap();
                println!("Closed remote {} at {}!", shake.peer_addr(), shake.local_addr());
            }
        }
        thread::spawn(move || {
            Builder::new()
                .bind("0.0.0.0:9").unwrap()
                .bind("0.0.0.0:9999").unwrap()
                .build(MyFactory).unwrap()
                .run().unwrap();
        });
        std::net::TcpStream::connect("127.0.0.1:9").unwrap();
        std::net::TcpStream::connect("127.0.0.1:9999").unwrap();
        Ok(())
    }
}
