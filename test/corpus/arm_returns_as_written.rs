// A `match` arm's returns as written, ReactDOMUnknownPropertyHook's
// `switch (typeof value)`: an early `return true;` of the value the arm
// ends with stays the `if` written before the rest, `if (prefix ===
// 'data-' || prefix === 'aria-') { return true; }`, not the rest inside
// its negation (rd48, ADR 0564, amended); and an inner `match` whose one
// arm that goes on does nothing keeps what's written after it after it,
// `switch (name) { case 'inert': break; default: return true; }
// console.error(..);` (rd49, ADR 0629, amended).
use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn error(s: &str) {
    LOG.with_borrow_mut(|l| l.push(s.to_string()));
}

enum Kind {
    Bool(bool),
    Text(String),
    Other,
}

fn check(name: &str, value: Kind) -> bool {
    match value {
        Kind::Bool(boolean) => match name {
            "checked" => {
                return true;
            }
            _ => {
                let prefix = &name[..name.len().min(5)];
                if prefix == "data-" || prefix == "aria-" {
                    return true;
                }
                if boolean {
                    error("yes");
                } else {
                    error("no");
                }
                error(name);
                return true;
            }
        },
        Kind::Text(value) if value == "true" => {
            match name {
                "checked" | "inert" => {}
                _ => {
                    return true;
                }
            }
            error("boolean");
            return true;
        }
        _ => {}
    }
    error("other");
    true
}

fn main() {
    for name in ["checked", "data-x", "aria-y", "hidden", "inert"] {
        let kinds = [
            Kind::Bool(name.len() > 5),
            Kind::Text("true".to_string()),
            Kind::Text("no".to_string()),
            Kind::Other,
        ];
        for kind in kinds {
            print!("{} ", check(name, kind));
        }
        println!("{:?}", LOG.with_borrow_mut(std::mem::take));
    }
}
