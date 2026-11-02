// A let chain after a test of its own, with an `else` and statements after
// it, whose binding is a read through `Option`s, `options.and_then(|o|
// o.debug_channel.as_ref()).and_then(|c| c.readable)`: one `if` of its
// tests and its `else`, the binding read in its body, `if (DEV &&
// options?.debugChannel?.readable != null) { const readable =
// options.debugChannel.readable; .. } else { .. } return getRoot(..)`, as
// React Flight's createFromReadableStream writes it, not the nested `if`s
// whose `else` is the tail copied into an early `return` (rd614 b, ADR
// 0842, amended).
pub struct Channel {
    pub readable: Option<u32>,
}

pub struct Options {
    pub debug_channel: Option<Channel>,
}

const DEV: bool = true;

fn get_root(r: &[u32]) -> u32 {
    r.iter().sum()
}

fn read_from(options: Option<&Options>, response: &mut Vec<u32>) -> u32 {
    if DEV && let Some(readable) = options.and_then(|o| o.debug_channel.as_ref()).and_then(|c| c.readable) {
        response.push(readable);
        response.push(readable + 1);
    } else {
        response.push(0);
    }
    get_root(response)
}

// A binding tested set, then a test of what it holds: past the test of
// its being set, `options?.debugChannel != null`, read with a `.`,
// `options.debugChannel.readable > 2`.
fn over_two(options: Option<&Options>, log: &mut Vec<u32>) -> usize {
    if DEV
        && let Some(channel) = options.and_then(|o| o.debug_channel.as_ref())
        && channel.readable.is_some_and(|r| r > 2)
    {
        log.push(channel.readable.unwrap_or(0));
    } else {
        log.push(1);
    }
    log.len()
}

// Two bindings tested one after the other: the second read with its `?.`
// until its own test holds, `b?.debugChannel?.readable != null`, where
// `b` may be `None` though `a` isn't.
fn both(a: Option<&Options>, b: Option<&Options>, log: &mut Vec<u32>) -> usize {
    if DEV
        && let (Some(x), Some(y)) = (
            a.and_then(|o| o.debug_channel.as_ref()).and_then(|c| c.readable),
            b.and_then(|o| o.debug_channel.as_ref()).and_then(|c| c.readable),
        )
    {
        log.push(x + y);
    } else {
        log.push(1);
    }
    log.len()
}

// A chain of no `else` that takes apart what its first test tested,
// joined by `and tests` (ADR 0842): its binding read with a `.` too,
// `const r = a.debugChannel.readable;`.
fn taken_apart(a: Option<&Options>, log: &mut Vec<u32>) {
    if let Some(o) = a
        && let Some(r) = o.debug_channel.as_ref().and_then(|c| c.readable)
    {
        log.push(r);
        log.push(r + 1);
    }
}

fn main() {
    let none = Options { debug_channel: None };
    let empty = Options {
        debug_channel: Some(Channel { readable: None }),
    };
    let full = Options {
        debug_channel: Some(Channel { readable: Some(4) }),
    };
    let mut response = Vec::new();
    for options in [None, Some(&none), Some(&empty), Some(&full)] {
        println!("{}", read_from(options, &mut response));
    }
    println!("{response:?}");
    let low = Options {
        debug_channel: Some(Channel { readable: Some(2) }),
    };
    let mut log = Vec::new();
    for options in [None, Some(&none), Some(&low), Some(&full)] {
        println!("{}", over_two(options, &mut log));
    }
    println!("{log:?}");
    let pairs = [
        (None, None),
        (Some(&full), None),
        (Some(&full), Some(&empty)),
        (Some(&full), Some(&low)),
    ];
    for (a, b) in pairs {
        println!("{}", both(a, b, &mut log));
    }
    println!("{log:?}");
    for options in [None, Some(&none), Some(&empty), Some(&full)] {
        taken_apart(options, &mut log);
    }
    println!("{log:?}");
}
