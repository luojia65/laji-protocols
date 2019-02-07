use std::{thread, net::*};

fn main() {
    for i in 0..10000 {
        let addr = match i % 3 {
            0 => "127.0.0.1:9",
            1 => "127.0.0.1:999",
            2 => "127.0.0.1:9999",
            _ => unreachable!(),
        };
        thread::spawn(move || {
            // thread::sleep(std::time::Duration::from_millis(2000-i));
            let _ = TcpStream::connect(addr).unwrap();
            println!("#{}, {}", i, addr);
        });
    }
    thread::sleep(std::time::Duration::from_millis(5000));
}