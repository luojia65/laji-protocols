use laji_protocols::discard_mio as discard;

struct MyFactory;
impl discard::Factory for MyFactory {
    type Handler = MyHandler;
    fn connection_made(&mut self) -> MyHandler {
        MyHandler(None)
    }
}
struct MyHandler(Option<discard::Handshake>);
impl discard::Handler for MyHandler {
    fn on_open(&mut self, shake: discard::Handshake) {
        println!("[{} -> {}]: Open!", shake.peer_addr(), shake.local_addr());
        self.0 = Some(shake);
    }
    fn on_close(&mut self) {
        let shake = self.0.unwrap();
        println!("[{} -> {}]: Close!", shake.peer_addr(), shake.local_addr());
    }
}

fn main() {
    discard::Builder::new()
        .bind("0.0.0.0:9").unwrap()
        .bind("0.0.0.0:999").unwrap()
        .bind("0.0.0.0:9999").unwrap()
        .build(MyFactory).unwrap()
        .run().unwrap();
}
