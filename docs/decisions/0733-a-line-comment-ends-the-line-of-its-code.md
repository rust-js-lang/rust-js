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

## Amendment: a comment after an argument's closing brackets alone (2026-11-02)

React Flight's browser client gives createResponse an `if` as an
argument, as rustfmt writes it:

```rust
createResponse(
    ..,
    if __DEV__ {
        options.is_none_or(|o| o.replay_console_logs != Some(false))
    } else {
        false
    }, // defaults to true
    ..,
)
```

The comment's line is the `else` block's `}` and the argument's `,`
alone, which no JS maps to, so it was nobody's and was lost (rd615).
**A line comment after only an argument's closing brackets and its `,`
is placed by the argument's lines, from the one its last bracket opened
on, past an `else`'s `} else {` to its `if`** (`trailing::opened_on`,
told by `to_oxc`'s comments): after the argument's last JS, the rest of
the JS line on the next, as above, `__DEV__ && (..), // defaults to true`,
upstream's; a last argument's, a field's value's and an array item's
too. A bracket inside a string or a comment between is read as code, so
the comment is placed by another line or lost, as before.

Held by lowering-comments-ends' "a comment after an argument's closing
brackets is after its JS", and the mutations `opened-on-unread`
(`to_oxc.ts`), `argument-lines-unread`, `argument-end-unseen` and
`opened-on-an-else` (`to_oxc/trailing.ts`); that one after code isn't
told an argument's lines, by `trailing.rs`'s unit test
`opened_on_is_of_closing_brackets_alone`.
