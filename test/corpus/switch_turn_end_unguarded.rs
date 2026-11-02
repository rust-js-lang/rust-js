// A `match` of strings that ends a loop's turn, with no guard, its arms
// `continue`, do nothing, or test in their bodies, `_ => { if let
// Some(held) = held { .. } }` of a value one arm moves: a `switch` of the
// arms, each that did nothing a `break`, as React DOM's
// clearContainerSparingly writes it, not an `if` of their tests negated
// (rd184, rd619, ADR 0636).
use std::rc::Rc;

fn clear(items: &[(&str, Option<Rc<String>>)]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some((name, held)) = items.get(i).cloned() {
        i += 1;
        match name {
            "HTML" | "HEAD" | "BODY" => {
                out.push(format!("nested {name}"));
                continue;
            }
            "SCRIPT" | "STYLE" => {
                continue;
            }
            "LINK" => {}
            _ => {
                if let Some(held) = held {
                    out.push(format!("kept {held}"));
                }
            }
        }
    }
    out
}

fn main() {
    let sheet = Rc::new("sheet".to_string());
    println!(
        "{:?}",
        clear(&[
            ("HTML", None),
            ("SCRIPT", None),
            ("LINK", Some(Rc::clone(&sheet))),
            ("LINK", None),
            ("DIV", None),
        ])
    );
}
