// An `Option<bool>` compared with `Some(true)` or `Some(false)`, or matched
// against one, is that value exactly, `o === true`, as React tests an
// option a caller may give, `options.unstable_strictMode === true`; its
// truth, `unwrap_or(false)`, stays `if (o)` (the user's decision, ADR 0298
// amended).
struct Options {
    strict_mode: Option<bool>,
    hydrate: Option<bool>,
}

fn strict(options: &Options) -> bool {
    options.strict_mode == Some(true)
}

fn not_hydrating(options: &Options) -> bool {
    options.hydrate == Some(false)
}

fn describe(flag: Option<bool>) -> &'static str {
    if let Some(true) = flag {
        "on"
    } else if flag != Some(false) {
        "unset"
    } else {
        "off"
    }
}

fn kind(flag: Option<bool>) -> &'static str {
    match flag {
        Some(true) => "yes",
        Some(false) => "no",
        None => "maybe",
    }
}

fn loose(flag: Option<bool>) -> bool {
    flag.unwrap_or(false)
}

fn main() {
    for (strict_mode, hydrate) in [(Some(true), Some(false)), (Some(false), None), (None, Some(true))] {
        let options = Options { strict_mode, hydrate };
        println!("{} {}", strict(&options), not_hydrating(&options));
    }
    for flag in [Some(true), Some(false), None] {
        println!("{} {} {} {}", describe(flag), kind(flag), loose(flag), matches!(flag, Some(false)));
    }
}
