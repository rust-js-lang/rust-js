# The pilot: a full-stack contacts app

ROADMAP M3.3. One Cargo workspace, three crates, and a Vite app:

```
examples/pilot/
├── package.json  rust-js's packages and crates, from npm, by version
├── Cargo.toml    the workspace: the three crates below
├── models        Contact, NewContact, Problem, and validate()  ◄── both sides
├── server        native Rust, std::net and serde_json:  /api/contacts
├── frontend      React, compiled by rust-js as Cargo checks it (ADR 0101)
└── web           Vite: imports `rust-js:frontend`, proxies /api to the server
```

It's an app of its own, as one `bun create @rust-js` makes is, and not part
of this repository's workspace: rust-js's packages and its crates, `js`,
`webapi` and `react`, come from npm, named by version in `package.json`
and `frontend/Cargo.toml`, and `rust-js-patch`, its `postinstall`, tells
Cargo they're in `node_modules` (ADR 0118).

The JS rust-js writes is committed beside the Rust it's from, as ReScript's
projects do (ADR 0041): [`frontend/src/api.js`](frontend/src/api.js) is
[`api.rs`](frontend/src/api.rs)'s, and `models/src/lib.js` the models'. A
change to the Rust is committed with its JS, which the test checks.

The client and the server share `models`: the JSON each sends is its
serde derives, and `validate` is the rule both hold a contact to. The form
checks before it sends; the server checks again, for requests that aren't
the form's, and knows what the form can't, that an email is taken.

```
browser ──► Vite ── /api ──► server ── models::validate
   │                            │
   └── frontend (rust-js) ──────┴── models (serde) ── the same JSON, both ways
```

## Run it

```bash
cd examples/pilot
bun install
cargo +1.99.0 run -p server    # http://127.0.0.1:3000
bun run dev                    # in another terminal
```

Deployed, the server serves the built client beside its API, one process:

```bash
bun run build                  # web/dist/
DIST=web/dist cargo +1.99.0 run --release -p server    # http://127.0.0.1:3000
```

The compiler is `@rust-js/native`'s, which is macOS on Apple silicon's for
now (M4.1). Elsewhere, `RUST_JS_COMPILER` names one: `bun run build` in this
repository's root builds it, `target/debug/rust-js`.

What it does, each checked by [`test/pilot.test.ts`](../../test/pilot.test.ts)
in a browser, with the server running:

- **Routing:** `#/`, `#/contacts/3` and `#/new`, followed on `hashchange`.
- **A list, searched as you type.** Each search aborts the one before, so a
  slow answer to an older search never replaces a newer one's.
- **Loading and errors:** each page says it's loading, and why it has
  nothing: a contact that doesn't exist, or a server that can't answer,
  with a button to try again.
- **A form**, validated as the server validates, every field at once, and
  then the server's errors by field: `ada@example.com is taken`.
- **An npm component:** [Sonner](https://sonner.emilkowal.ski)'s `<Toaster>`
  and `toast()`, bound in [`frontend/src/sonner.rs`](frontend/src/sonner.rs).
- **The server refuses** JSON that isn't a contact (400) and a contact that
  breaks the rules (422), with its reasons.

And how it's developed and deployed (ROADMAP M3.4):

- **A save of the Rust is a Fast Refresh**: `list.rs`'s new heading shows,
  and the search typed stays.
- **A compile error is Vite's overlay**, and the app runs on as it last
  compiled; fixed, the overlay goes.
- **The browser's devtools show the Rust**: the module it runs maps back to
  `list.rs`, as it was saved, where its breakpoints are set.
- **The production build**, served by the native server with `DIST`, works
  as the dev server's does, and serves nothing outside `web/dist`.

## What it found

Fixed in the compiler, each with a test and a mutation:

- **A binding used as a value** (`.map(abort_signal::aborted)`) wasn't
  supported, and neither was **a package's component** as a JSX tag, which
  lowers to one (ADRs 0039, 0040).

Fixed in the binding crates, each with a test:

- **The web crate had no options objects**, no `fetch` with a
  `RequestInit`, no `addEventListener` with a signal, and nothing had
  `encodeURIComponent`: the pilot declared each itself. Now they're the
  `webapi` and `js` crates', named as ReScript's (ADR 0102), and the pilot
  declares no bindings but Sonner's.

Fixed in the compiler since, for M3.4: **iterating `str::bytes()`** (ADR
0126), which the pilot needed before it had `js::encode_uri_component`,
and **`Result::as_ref`**, for which the form used `.ok()`.

Fixed in the packages: **the bindings were a path into this repository**
(`../../../react`), and rust-js's packages its workspace's. They're on npm
now (M4.2), and the pilot names them by version.
