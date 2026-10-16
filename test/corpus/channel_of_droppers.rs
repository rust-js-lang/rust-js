// A channel of values with a destructor (ADR 0142): what's received is the
// receiver's to drop; what's still queued is dropped with the receiver, in
// the order it was sent; a send after it's gone gives the value back. And a
// channel of senders: the queued sender dropped with the receiver is one
// sender fewer of its own channel.
use std::sync::mpsc::{channel, Receiver, Sender};

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let (tx, rx) = channel();
    tx.send(Loud("a")).unwrap();
    tx.send(Loud("b")).unwrap();
    tx.send(Loud("c")).unwrap();
    {
        let first = rx.recv().unwrap();
        println!("got {}", first.0);
    }
    drop(rx);
    println!("receiver gone");
    match tx.send(Loud("d")) {
        Ok(()) => println!("sent"),
        Err(back) => println!("back {}", (back.0).0),
    }

    let (outer_tx, outer_rx): (Sender<Sender<u8>>, Receiver<Sender<u8>>) = channel();
    let (inner_tx, inner_rx) = channel::<u8>();
    outer_tx.send(inner_tx).unwrap();
    println!("{:?}", inner_rx.try_recv());
    drop(outer_rx);
    println!("{:?}", inner_rx.try_recv());
}
