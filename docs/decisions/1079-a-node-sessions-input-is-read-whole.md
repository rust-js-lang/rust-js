# 1079. A Node session's input is read whole, as one UTF-8 text

Status: Accepted (2026-11-04, #573, P1). Amends
[0207](0207-declarations-printed-by-typescript.md) and
[0381](0381-the-apps-prettier-lays-it-out.md): how each one's session
reads what it's given.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. No Rust and JS
semantics differ here. This is rust-js's own pipe between processes.

## Context

rust-js runs two Node sessions where the crate is: TypeScript's printer of
the `.d.ts` (ADR 0207) and the app's Prettier (ADR 0381). It gives each one
its JSON on stdin. Each script read that input with its own copy of the
same line:

```js
for await (const chunk of process.stdin) input += chunk;
```

A chunk is a `Buffer`, and `+=` decodes each one on its own. A pipe's
chunks are 64 kB, and a UTF-8 character split across two chunks was decoded
as two `U+FFFD`. React's port hit this in ReactFlightPerformanceTrack.js,
whose `'Server Components ⚛'` Prettier was given as `��`. rust-js's check
of Prettier's layout (`format::transformed`) saw a string change, warned,
and kept rust-js's own layout, 262 lines away from upstream's. TypeScript's
session had the same bug with no such check, so its `.d.ts` got the
replacement characters silently, in a doc comment or a non-ASCII name. A
build only hits it once its JSON passes 64 kB with a character on a
boundary, so it showed up in a fresh regen and nowhere earlier. Both
copies are as old as their sessions: addf48811 (TypeScript) and 927e08c36
(Prettier).

## Decision

**A Node session is `node_session`'s: the script reads its input by
`await given()`, which decodes stdin as one UTF-8 text
(`process.stdin.setEncoding("utf8")`), and writes its answer as JSON.**
`node_session::run` starts the session, writes the input while it's read
and parses the answer. `node_session::node()` is the Node that runs it
(`RUST_JS_NODE`). Each adapter keeps its own script and its own messages.

- One owner of how a session reads its input, in place of two copies that
  had the same bug. A third session gets it right by using `run`.
- Rejected: `setEncoding` added to each script, which fixes the bug and
  keeps the rule in two places.
- Rejected: collecting the Buffers and decoding them once, which is the
  same text for more code.

## Consequences

- test/settings.test.ts holds this with each session given 300 kB of
  characters of 3 and 4 bytes, and `e` with its combining accent, past
  several 64 kB chunks, so wherever the chunks fall one splits a character:
  "non-ASCII text across a Node session's stdin chunks is Prettier's
  whole" (its layout taken, the string as written) and "non-ASCII names
  and docs across a Node session's stdin chunks are TypeScript's whole" (a
  name of `名` and an astral `𠀀`, and a doc comment, with no Prettier
  config).
- Mutation `session-stdin-decoded-by-chunk` puts the per-chunk decoding
  back.
