use std::{
    borrow::Cow,
    io::{self, Write},
    net::{TcpListener, TcpStream, UdpSocket, SocketAddr, ToSocketAddrs},
    thread,
    sync::{mpsc, Arc, Mutex},
};

pub fn listen<A, F, H>(addr: A, factory: F) -> io::Result<()>
where 
    A: ToSocketAddrs, 
    F: FnMut(Sender) -> H, 
    F: 'static + Send + Sync,
    H: Handler 
{
    LajiDaytime::new(factory)
        .bind_tcp(&addr)?
        .bind_udp(&addr)?
        .run()
}

pub struct LajiDaytime<F> 
where F: Factory {
    tcp: Vec<TcpListener>,
    udp: Vec<UdpSocket>,
    factory: F
}

impl<F> LajiDaytime<F> 
where F: Factory {
    #[inline]
    pub fn new(factory: F) -> Self {
        Self {
            tcp: Vec::new(),
            udp: Vec::new(),
            factory
        }
    }

    #[inline]
    pub fn bind_tcp<A>(mut self, addr: A) -> io::Result<Self>
    where 
        A: ToSocketAddrs 
    {
        let listener = TcpListener::bind(addr)?;
        self.tcp.push(listener);
        Ok(self)
    }

    #[inline]
    pub fn bind_udp<A>(mut self, addr: A) -> io::Result<Self>
    where 
        A: ToSocketAddrs 
    {
        let socket = UdpSocket::bind(addr)?;
        self.udp.push(socket);
        Ok(self)
    }
}

impl<F> LajiDaytime<F> 
where 
    F: Factory + Send + Sync + 'static 
{
    pub fn run(self) -> io::Result<()> {
        let (err_tx, err_rx) = mpsc::channel();
        let factory = Arc::new(Mutex::new(self.factory));
        for listener in self.tcp { 
            let err_tx = err_tx.clone();
            let factory = Arc::clone(&factory);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let ans = || {
                        let stream = stream?;
                        let hs = Handshake::read_tcp_stream(&stream)?;
                        let sender = Sender::new_tcp(stream);
                        let mut handler = factory.lock().unwrap().connection_made(sender);
                        handler.on_open(hs);
                        handler.on_request();
                        handler.on_close();
                        Ok(())
                    };
                    ans().unwrap_or_else(|e: io::Error| err_tx.send(e).unwrap())
                }
            });   
        }
        for socket in self.udp {
            let err_tx = err_tx.clone();
            let factory = Arc::clone(&factory);
            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                let mut ans = || {
                    let (_size, addr) = socket.recv_from(&mut buf)?;
                    let hs = Handshake::from_udp_addr(addr);
                    let sender = Sender::new_udp(socket.try_clone()?, addr);
                    let mut handler = factory.lock().unwrap().connection_made(sender);
                    handler.on_open(hs);
                    handler.on_request();
                    handler.on_close();
                    Ok(())
                };
                loop {
                    ans().unwrap_or_else(|e: io::Error| err_tx.send(e).unwrap())
                }
            });
        }
        while let Ok(err) = err_rx.recv() {
            return Err(err);
        }
        Ok(())
    } 
}

pub trait Factory {
    type Handler: Handler; 

    fn connection_made(&mut self, _sender: Sender) -> Self::Handler;
}

impl<F, H> Factory for F 
where 
    H: Handler, 
    F: FnMut(Sender) -> H 
{
    type Handler = H;

    #[inline]
    fn connection_made(&mut self, sender: Sender) -> H {
        self(sender)
    }
}

pub enum Sender {
    Tcp {
        stream: TcpStream,
    },
    Udp {
        socket: UdpSocket,
        target: SocketAddr,
    }
}

impl Sender {
    #[inline]
    fn new_tcp(stream: TcpStream) -> Self {
        Sender::Tcp { stream }
    }

    #[inline]
    fn new_udp(socket: UdpSocket, target: SocketAddr) -> Self {
        Sender::Udp { socket, target }
    }

    #[inline]
    pub fn send<'m, M>(&mut self, msg: M) -> io::Result<usize>
    where M: Into<Cow<'m, str>> {
        let buf = msg.into().to_string().into_bytes();
        match self {
            Sender::Tcp { stream } => stream.write(&buf),
            Sender::Udp { socket, target } => socket.send_to(&buf, *target)
        }
    }
}

pub trait Handler {
    fn on_open(&mut self, _shake: Handshake) {}

    fn on_request(&mut self) {}

    fn on_close(&mut self) {}
}

impl<F> Handler for F
where 
    F: FnMut() 
{
    #[inline]
    fn on_request(&mut self) {
        self()
    }
}

impl<F1, F2> Handler for (F1, F2) 
where 
    F1: FnMut(Handshake), 
    F2: FnMut() 
{
    #[inline]
    fn on_open(&mut self, shake: Handshake) {
        self.0(shake)
    }

    #[inline]
    fn on_request(&mut self) {
        self.1()
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Handshake {
    Tcp {
        peer_addr: SocketAddr,
        local_addr: SocketAddr,
    },
    Udp {
        origin_addr: SocketAddr,
    }
}

impl Handshake {
    #[inline]
    fn read_tcp_stream(ts: &TcpStream) -> io::Result<Self> {
        Ok(Handshake::Tcp {
            peer_addr: ts.peer_addr()?,
            local_addr: ts.local_addr()?,
        })
    }

    #[inline]
    fn from_udp_addr(origin_addr: SocketAddr) -> Self {
        Handshake::Udp { origin_addr }
    }
}

#[cfg(test)]
mod tests {
    mod laji_daytime {
        pub use super::super::*;
    }
    use std::io;
    #[test]
    fn listen_one() -> io::Result<()> {
        laji_daytime::listen("0.0.0.0:13", move |mut out| {
            move || {
                out.send(time_string()).unwrap();
            }
        })
    }

    #[test]
    fn listen_batch() -> io::Result<()> {
        use super::*;
        struct MyFactory;
        impl Factory for MyFactory {
            type Handler = MyHandler;
            fn connection_made(&mut self, sender: Sender) -> MyHandler {
                MyHandler(sender)
            }
        }
        struct MyHandler(Sender);
        impl Handler for MyHandler {
            fn on_open(&mut self, shake: Handshake) {
                println!("Open! Shake: [{:?}]", shake);
            }
            fn on_request(&mut self) {
                let string = time_string();
                self.0.send(&string).unwrap();
                println!("Sent! [{:?}]", string);
            }
            fn on_close(&mut self) {
                println!("Closed!")
            }
        }
        LajiDaytime::new(MyFactory)
            .bind_tcp("0.0.0.0:13")?
            .bind_tcp("0.0.0.0:13333")?
            .bind_udp("0.0.0.0:13")?
            .bind_udp("0.0.0.0:13333")?
            .run()
            .expect("Error occurred!");
        Ok(())
    }

    #[inline]
    fn time_string() -> String {
        chrono::offset::Local::now().to_rfc2822()
    }

}
