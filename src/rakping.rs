use std::{io, net};
use std::borrow::Cow;
use core::ptr;

pub fn listen<A, F, H>(addr: A, factory: F) -> io::Result<()> 
where
    A: net::ToSocketAddrs,
    F: FnMut(Sender) -> H,
    H: Handler
{
    Ok(())
}

pub fn connect<A, F, H>(addr: A, factory: F) -> io::Result<()>
where
    A: net::ToSocketAddrs,
    F: FnMut(Sender) -> H,
    H: Handler
{
    Ok(())
}

pub struct LajiRakPing<'a, F> 
where F: Factory
{
    remote_socket: &'a [net::UdpSocket],
    local_socket: Option<net::UdpSocket>,
    factory: F
}

pub trait Factory {
    type Handler: Handler;

    fn connection_made(&mut self, sender: Sender) -> Self::Handler;

    #[inline]
    fn client_connected(&mut self, sender: Sender) -> Self::Handler {
        self.connection_made(sender)
    }

    #[inline]
    fn server_connected(&mut self, sender: Sender) -> Self::Handler {
        self.connection_made(sender)
    }
}

pub trait Handler {
    fn on_ping(&mut self, ping: &Ping) -> io::Result<()>;

    fn on_pong(&mut self, pong: &Pong) -> io::Result<()>;
}

#[derive(Clone)]
pub struct Sender<'a> {
    socket: &'a net::UdpSocket,
    addr: net::SocketAddr,
}

impl<'a> Sender<'a> {
    fn new(socket: &'a net::UdpSocket, addr: net::SocketAddr) -> Self {
        Self { socket, addr }
    }
}

impl Sender<'_> {
    pub fn send_ping(&self, ping: &Ping) -> io::Result<usize> {
        let mut buf = [0u8; 17];
        buf[0] = 0x01;
        unsafe { 
            *(buf.as_ptr().offset(1) as *mut u64) = ping.ping_time.to_be();
            *(buf.as_ptr().offset(9) as *mut u64) = ping.client_guid.to_be(); 
        }
        self.socket.send_to(&buf, self.addr)
    }

    pub fn send_pong(&self, pong: &Pong) -> io::Result<usize> {
        let mut buf = [0u8; 1024];
        buf[0] = 0x1c;
        let len_server_name = pong.server_name.len();
        unsafe {
            let buf_ptr = buf.as_ptr();
            *(buf_ptr.offset(1) as *mut u64) = pong.ping_time.to_be();
            *(buf_ptr.offset(9) as *mut u64) = pong.server_guid.to_be(); 
            *(buf_ptr.offset(17) as *mut u16) = len_server_name as u16;
            ptr::copy_nonoverlapping(pong.server_name.as_ptr(), buf_ptr.offset(19) as *mut u8, len_server_name);
        }
        let len = len_server_name + 19;
        self.socket.send_to(&buf[..len], self.addr)
    }
}

pub struct Ping {
    ping_time: u64,
    client_guid: u64,
}

impl Ping {
    #[inline]
    pub fn new(ping_time: u64, client_guid: u64) -> Self {
        Self { ping_time, client_guid }
    }
}

pub struct Pong<'a> {
    ping_time: u64,
    server_guid: u64,
    server_name: Cow<'a, str>,
}

impl<'a> Pong<'a> {
    #[inline]
    pub fn new<S>(ping_time: u64, server_guid: u64, server_name: S) -> Self 
    where S: Into<Cow<'a, str>> {
        Self { ping_time, server_guid, server_name: server_name.into() }
    }
}
