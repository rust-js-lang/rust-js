// A discriminated union's `otherwise` variant is the object it holds (ADR
// 0284): a match binding it, `Other(note) => note.text`, reads that value
// itself, where it read a `_0` of it, `undefined`, and threw (#2396); a
// `&mut` binding writes the object in place, one taken out is it, and its
// drop runs as the value holding it is dropped.
struct Note {
    text: String,
    id: u32,
}

impl Drop for Note {
    fn drop(&mut self) {
        println!("drop note {} {}", self.id, self.text);
    }
}

#[cfg_attr(rust_js, rust_js::tag = "type")]
enum Message {
    #[cfg_attr(rust_js, rust_js::name = "ping")]
    Ping { at: u32 },
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(Note),
}

fn described(message: &Message) -> String {
    match message {
        Message::Ping { at } => format!("ping {at}"),
        Message::Other(note) => format!("other {} {}", note.text, note.id),
    }
}

fn taken(message: Message) -> Option<Note> {
    match message {
        Message::Other(note) => Some(note),
        Message::Ping { .. } => None,
    }
}

fn shouted(message: &mut Message) {
    if let Message::Other(note) = message {
        note.text.push('!');
    }
}

fn main() {
    let mut all = vec![
        Message::Other(Note { text: "a".into(), id: 1 }),
        Message::Ping { at: 2 },
        Message::Other(Note { text: "b".into(), id: 3 }),
    ];
    for message in &all {
        println!("{}", described(message));
    }
    for message in &mut all {
        shouted(message);
    }
    for message in &all {
        println!("{}", described(message));
    }
    let first = all.remove(0);
    if let Some(note) = taken(first) {
        println!("taken {}", note.text);
    }
    let ping = all.remove(0);
    println!("{:?}", taken(ping).map(|note| note.id));
    println!("dropping the rest");
    drop(all);
    println!("end");
}
