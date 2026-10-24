# 0731. A case's `break` past the `switch` is the case's own

Status: Accepted. Amends [0629](0629-a-match-of-constants-is-a-switch.md):
a case ends in its own `break` where a block's would go to the same place.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
which `break` leaves for where both go is how the JS reads.

## Context

React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
gives `desc` of an `if` whose `else` is a `match` of what the value is:

```rust
let desc = if let Some(value) = value
    && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(value))
{
    ..
} else {
    match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::Function(function) => match js::classify(js::get(js::unknown(function), "name")) {
            Kind::String(name) if !name.is_empty() => name.to_string() + "() {}",
            _ => "() => {}".to_string(),
        },
        ..
    }
};
properties.push((.., desc));
```

Where the `if` and the `match` end is one MIR block, so the inner match's
first arm jumped there by a label MIR made around the whole `if`:

```js
let desc;
bb374: {
  if (..) { .. } else if (value === null) { .. } else {
    switch (typeof value) {
      case "function": {
        ..
        if (typeof functionName === "string" && functionName) {
          desc = functionName + "() {}";
          break bb374;
        }
        ..
```

## Decision

**A block's `break` in a case of a `switch` the block's statements end
with, through the branches of the `if`s that end them, is the case's own
`break`**: nothing runs between where the `switch` ends and where the
block does (`switches::breaks_past_switches`, after `switches`). One in a
loop of the case stays the block's, as a bare `break` would leave the
loop. The block's label, named by nothing then, goes.

## Consequences

- The port's `case 'function'` ends in `break;`, and its `bb374` is gone.
- A program's own label, `'desc: { .. break 'desc x; .. }`, is still
  named as MIR names its block, `bb28`, where a `break` of it stays; that
  it keeps its name is open.
