# 0733. A line comment ends the line of its code, where JS has more after it

Status: Accepted. Amends [0444](0444-a-comment-is-where-its-code-is.md),
whose comment after code is after the last JS of its line.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
where a comment is changes what the JS says, not what it does.

## Context

React's addValueToProperties (shared/ReactPerformanceTrackProperties.js)
says what each `…` is:

```rust
Kind::String(value) => {
    if value == OMITTED_PROP_ERROR {
        "\u{2026}".to_string() // ellipsis
    } else {
        js::json::stringify(&if value.encode_utf16().count() >= 1024 {
            js::string::slice(value, 0.0, Some(1023.0)) + "\u{2026}" // ellipsis
        } else {
            value.to_string()
        })
    }
}
```

Both branches are a value, one conditional on one JS line, and a comment
after code went after the last JS of its line: both after the whole of
it, `..: value); // ellipsis // ellipsis`, neither by its `…`.

## Decision

**A line comment after code, whose code's JS a later line's follows on
its JS line, is after the JS of the code right before it, and the rest of
that JS line goes on the next** (`to_oxc`'s `ends`): `? "…" // ellipsis`,
the formatter laying out what follows. Where nothing of a later line
follows, it's after the line's last JS, as before. A block comment inside
an expression is where ADR 0630 put it.

## Consequences

- The port's two `// ellipsis` are each after its `"…"`, as upstream's.
