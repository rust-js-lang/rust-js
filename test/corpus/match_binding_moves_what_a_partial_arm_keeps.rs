// #2064: a `match` of a value one arm takes a part of, `Kind::Text(bytes)`,
// and a catch-all binds whole and moves on, `kind => new.push(Node { kind
// })`: after the match, only the first arm leaves what wasn't moved to drop,
// never the value the catch-all moved. regex-syntax's `Hir::concat` dropped
// the moved `kind2` after its match, emptying each `Hir` it had just pushed.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

enum Kind {
    Text(Vec<u8>),
    Group(Vec<Node>),
    Mark(Loud),
}

struct Node {
    kind: Kind,
}

fn flatten(subs: Vec<Node>) -> Vec<Node> {
    let mut new = vec![];
    let mut text: Option<Vec<u8>> = None;
    for sub in subs {
        match sub.kind {
            Kind::Group(subs2) => {
                for sub2 in subs2 {
                    let kind2 = sub2.kind;
                    match kind2 {
                        Kind::Text(bytes) => text.get_or_insert_with(Vec::new).extend_from_slice(&bytes),
                        kind2 => {
                            if let Some(bytes) = text.take() {
                                new.push(Node { kind: Kind::Text(bytes) });
                            }
                            new.push(Node { kind: kind2 });
                        }
                    }
                }
            }
            kind => new.push(Node { kind }),
        }
    }
    if let Some(bytes) = text {
        new.push(Node { kind: Kind::Text(bytes) });
    }
    new
}

// The same shape as an `if let` whose `else` moves it whole, and a `let
// else` whose `else` does.
fn texts_if_let(subs: Vec<Node>) -> (Vec<u8>, Vec<Node>) {
    let (mut text, mut kept) = (vec![], vec![]);
    for sub in subs {
        let kind = sub.kind;
        if let Kind::Text(bytes) = kind {
            text.extend(bytes);
        } else {
            kept.push(Node { kind });
        }
    }
    (text, kept)
}

fn texts_let_else(subs: Vec<Node>) -> (Vec<u8>, Vec<Node>) {
    let (mut text, mut kept) = (vec![], vec![]);
    for sub in subs {
        let kind = sub.kind;
        let Kind::Text(bytes) = kind else {
            kept.push(Node { kind });
            continue;
        };
        text.extend(bytes);
    }
    (text, kept)
}

fn nodes() -> Vec<Node> {
    vec![
        Node { kind: Kind::Text(b"a".to_vec()) },
        Node { kind: Kind::Mark(Loud(4)) },
        Node { kind: Kind::Text(b"b".to_vec()) },
    ]
}

fn show(nodes: &[Node]) -> String {
    let parts: Vec<String> = (nodes.iter())
        .map(|node| match &node.kind {
            Kind::Text(bytes) => format!("text {}", String::from_utf8_lossy(bytes)),
            Kind::Group(subs) => format!("group of {}", subs.len()),
            Kind::Mark(loud) => format!("mark {}", loud.0),
        })
        .collect();
    parts.join(", ")
}

fn main() {
    let group = Node {
        kind: Kind::Group(vec![
            Node { kind: Kind::Text(b"<".to_vec()) },
            Node { kind: Kind::Mark(Loud(1)) },
            Node { kind: Kind::Text(b"x".to_vec()) },
            Node { kind: Kind::Mark(Loud(2)) },
        ]),
    };
    let flat = flatten(vec![group, Node { kind: Kind::Mark(Loud(3)) }]);
    println!("{}", show(&flat));
    let (text, kept) = texts_if_let(nodes());
    println!("if let: {} and {}", String::from_utf8_lossy(&text), show(&kept));
    let (text, kept) = texts_let_else(nodes());
    println!("let else: {} and {}", String::from_utf8_lossy(&text), show(&kept));
    println!("end");
}
