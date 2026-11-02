// An `Option`'s `map` of a field of a field of plain Rust data,
// `debug_channel.writable.map(callback_of)` of a binding a let-chain made
// of `options.debug_channel`: read twice where it is,
// `options.debugChannel.writable != null ? callbackOf(..) : undefined`, as
// a field of a variable is, not first in a `const option`, so the `if`
// that gives a value is the conditional upstream writes, React Flight's
// browser client's createResponseFromOptions's `const debugChannel =
// __DEV__ && options && options.debugChannel !== undefined ? {..} :
// undefined;` (rd614).
use std::rc::Rc;

struct Channel {
    readable: Option<u32>,
    writable: Option<u32>,
}

struct Options {
    debug_channel: Option<Channel>,
    name: Option<String>,
}

#[derive(Debug)]
struct Debug {
    has_readable: bool,
    callback: Option<u32>,
}

const DEV: bool = true;

fn callback_of(w: u32) -> u32 {
    w + 1
}

fn debug_of(options: Option<Options>) -> Option<Rc<Debug>> {
    let debug_channel = if DEV
        && let Some(options) = options
        && let Some(debug_channel) = options.debug_channel
    {
        Some(Rc::new(Debug {
            has_readable: debug_channel.readable.is_some(),
            callback: debug_channel.writable.map(callback_of),
        }))
    } else {
        None
    };
    debug_channel
}

fn main() {
    let channel = |readable, writable| Channel { readable, writable };
    for options in [
        None,
        Some(Options {
            debug_channel: None,
            name: None,
        }),
        Some(Options {
            debug_channel: Some(channel(Some(1), None)),
            name: Some("a".to_string()),
        }),
        Some(Options {
            debug_channel: Some(channel(None, Some(4))),
            name: None,
        }),
    ] {
        let named = options.as_ref().and_then(|o| o.name.clone());
        println!("{:?} {named:?}", debug_of(options));
    }
}
