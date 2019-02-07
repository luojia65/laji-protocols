use std::{
    io,
    net::{ToSocketAddrs, TcpListener, TcpStream, SocketAddr},
    thread,
    sync::{mpsc, Arc, Mutex},
};

pub fn listen<A, F, H>(addr: A, factory: F) -> io::Result<()>
where 
    A: ToSocketAddrs, 
    F: FnMut() -> H,
    F: Send + Sync + 'static,
    H: Handler 
{
    Builder::new().bind(addr)?.build(factory).run()
}

#[derive(Debug)]
pub struct LajiDiscard<F>
where F: Factory
{
    tcp: Vec<TcpListener>,
    factory: F
}

impl<F> LajiDiscard<F>
where   
    F: 'static + Factory + Send + Sync 
{
    pub fn run(self) -> io::Result<()> {
        let (err_tx, err_rx) = mpsc::channel();
        let factory = Arc::new(Mutex::new(self.factory));
        for listener in self.tcp {
            let err_tx = err_tx.clone();
            let listener = listener.try_clone()?;
            let factory = Arc::clone(&factory);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    process_one_stream(factory.clone(), stream)
                        .unwrap_or_else(|e| err_tx.send(e).unwrap())
                }
            });
        }
        while let Ok(err) = err_rx.recv() {
            return Err(err);
        }
        Ok(())
    }
}

fn process_one_stream<F>(factory: Arc<Mutex<F>>, stream: io::Result<TcpStream>) -> io::Result<()> 
where F: Factory
{
    let mut handler = factory.lock().unwrap().connection_made();
    let stream = stream?;
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
    pub fn new() -> Self {
        Self { tcp: Vec::new() }
    }

    pub fn bind<A>(mut self, addr: A) -> io::Result<Builder> 
    where A: ToSocketAddrs 
    {
        let new_listener = TcpListener::bind(addr)?;
        self.tcp.push(new_listener);
        Ok(self)
    }

    pub fn build<F>(self, factory: F) -> LajiDiscard<F> 
    where F: Factory
    {
        LajiDiscard {
            tcp: self.tcp,
            factory,
        }
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

    fn on_close(&mut self) {}
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
                .build(MyFactory)
                .run().unwrap();
        });
        TcpStream::connect("127.0.0.1:9").unwrap();
        TcpStream::connect("127.0.0.1:9999").unwrap();
        Ok(())
    }
}
