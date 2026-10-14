# 0378. A Next.js app moves to Rust one file at a time

Status: Proposed. The design we want; what it takes to deliver it is
measured against it, not the other way round. Would change
[0192](0192-next.md), [0196](0196-typescript-declarations.md),
[0046](0046-camel-case-crates.md) and [0028](0028-js-module-imports.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how an app takes
rust-js in, not what a program means.

## Context

Nobody rewrites a working app. A team that adopts rust-js has a Next.js app
in TypeScript, a deploy that works, and teammates who don't write Rust. It
moves one file, sees it work, and moves the next, for months.

Today that first file costs a day. Nothing documents adding rust-js to an
existing app, and only a new one can be made, with packages npm doesn't
have. Then `dev` and `build` become `rust-js-next`, which can't wrap a
`dev` script of the app's own, react.dev's `next-remote-watch`; a patch
step and a second copy of the app's Prettier config are written by hand,
and the `.d.ts` fails the app's format check. A Rust file that imports a TS one hand-writes
its binding, unchecked, from the crate root's directory. A TS file that
imports a Rust one sees `.d.ts` only if a flag says so, and snake_case
names unless another does.

## Decision

**A file's language is invisible to everything that imports it.** Moving a
file to Rust is writing `X.rs`, naming it in the crate's root, and deleting
`X.tsx`. Nothing else changes: not its importers, TS or Rust, not the
scripts, not the deploy.

### 1. Adding rust-js is one dependency and one line

```js
// next.config.js
const {withRustJs} = require('@rust-js/next-plugin');
module.exports = withRustJs(nextConfig);
```

`withRustJs` takes no options and changes nothing of the config it's
given: the webpack function, the aliases and `pageExtensions` are the
app's, and the JS it serves is the JS on disk. It acts by the phase Next.js
loads the config in, so whatever starts Next.js starts it, `next dev`,
`next build`, a custom server, react.dev's `next-remote-watch`, and the
`dev` and `build` scripts stay as they are:

- **Developing**: it compiles the crate once, then on each save of a `.rs`
  file, one watcher however many times the config is loaded.
- **Building**: it checks the committed JS is what the Rust compiles to
  (rule 4), and compiles nothing.
- **Anything else**, `next start`, `next lint`: nothing.

For now the packages are linked from a rust-js checkout, not installed
from npm, the compiler is Apple silicon's, and Rust is rustup's pinned
toolchain: the app's `package.json` names `@rust-js/next-plugin` and
`@rust-js/runtime` by `link:` path, and its `Cargo.toml` the binding
crates by `path`.

### 2. The app's Rust is one crate, and its root lists every file

```toml
[package]
name = "react-dev"
edition = "2024"

[lib]
path = "src/lib.rs"

[dependencies]
js = {package = "rust-js-builtins", path = "../rust-js/builtins"}
react = {package = "rust-js-react", path = "../rust-js/react"}
next = {package = "rust-js-next", path = "../rust-js/next"}
```

```rust
// src/lib.rs: the app's Rust, each module a file of it.
#[path = "components/ExternalLink.rs"]
pub mod external_link;
```

The root is the one list of the app's modules that Rust sees: rustc,
rust-analyzer and rust-js read it as they read any crate's. It writes no
JS of its own. A file moved to Rust is a line in it.

One file in, one file out: `ExternalLink.rs` compiles to
`ExternalLink.js` and `ExternalLink.d.ts` beside it, at the path the `.tsx`
had, its `mod x { .. }` blocks inside it.

### 3. Importers don't change

**A TS file importing a Rust one** reads its `.d.ts`, always written, with
no key to turn it on, and
the names JS gives: `pub fn format_date` is `formatDate`, as ADR 0376's one
naming rule reads a JS name back. `import {ExternalLink} from
'components/ExternalLink'` resolves, type-checks and renders as before.

**A Rust file importing a TS one** writes what it would for a Rust one,
the TS module named in the root as a Rust one is, by its file:

```rust
#[path = "components/Button.tsx"]
pub mod button;
```

```rust
use crate::button::Button;
```

rust-js binds each TS module a Rust file imports from TypeScript's own
types of it, the reverse of what `.d.ts` writes, and nobody writes a
binding of the app's code. A type Rust can't hold as TypeScript types it is
an error at the import, never a guess narrower or looser. When the TS
changes, rustc checks the Rust against it; when its file moves to Rust, its
Rust importers don't notice: the root's line says `Button.rs`.

**A package** is imported by its name, `use react::use_state;`, bound by
rust-js's own crate where it has one, React's and Next.js's, and from the
package's types where not. **A file that isn't code**, a CSS module, an
image or JSON, is a module typed as Next.js types it: a CSS module's class
names, an image's `StaticImageData`.

### 4. The JS is committed; only who writes Rust needs Rust

`next dev` compiles a `.rs` file on save, and Fast Refresh keeps the page's
state. A compile error shows where a TS one does: in the browser's overlay
and the terminal, at the Rust line.

The JS and `.d.ts` are committed beside the Rust, so `next build` builds
what's there and never compiles Rust: a teammate, CI and Vercel need no
toolchain. Each JS file says what it was written from, so a `next build`
whose Rust changed since stops, naming the file and the one command that
writes it fresh.

The JS reads as the app's own: it and its `.d.ts` are formatted by the
app's Prettier config, `.prettierrc` read where it is, so the app's format
check, react.dev's `prettier --list-different`, passes on them as written.

### 5. A file is Rust or TypeScript, never both

`ExternalLink.rs` beside `ExternalLink.tsx` is an error naming both:
delete one. rust-js writes only files it wrote; a JS file a person wrote,
at the path a Rust file's JS would take, is an error, never overwritten.

## The first file

react.dev's `src/components/ExternalLink.tsx`, imported by five TS and JS
files, some as `components/ExternalLink`, some as `../ExternalLink`:

```tsx
export function ExternalLink({href, target, children, ...props}: DetailedHTMLProps<AnchorHTMLAttributes<HTMLAnchorElement>, HTMLAnchorElement>) {
  return <a href={href} target={target ?? '_blank'} rel="noopener" {...props}>{children}</a>;
}
```

With rust-js linked, `Cargo.toml` and `src/lib.rs` written and the one
line in `next.config.js`, the author writes
`src/components/ExternalLink.rs`, names it in `src/lib.rs`, deletes the
`.tsx`, and saves:

```rust
use react::attributes::AnchorHTMLAttributes;
use react::{JSX, ReactNode, jsx};

pub struct ExternalLinkProps<'a, C: ReactNode> {
    pub children: C,
    #[rust_js::flatten]
    pub props: AnchorHTMLAttributes<'a>,
}

pub fn ExternalLink<C: ReactNode>(ExternalLinkProps { children, props }: ExternalLinkProps<'_, C>) -> JSX::Element {
    let target = props.target.unwrap_or("_blank");
    jsx! { <a {...props} target={target} rel="noopener">{children}</a> }
}
```

The page reloads with the link the Rust renders. The five importers, the
scripts and the deploy are as they were. The commit is the `.rs`, its
`.js` and `.d.ts`, the deleted `.tsx`, and the root's line; the first one
also `Cargo.toml`, `src/lib.rs`, the two links and the line in
`next.config.js`.

## Consequences

- **What goes**, each a second way once this is delivered: `rust-js-next`
  and its scripts; the `declarations` key; `js::camel_case!()`; a
  hand-written binding of the app's own modules, and its root-relative
  path; `[package.metadata.rust-js.format]`, a second copy of the app's
  Prettier config.
- **Later, not now**: one `rust-js` package on npm in place of the links;
  the compiler for Linux, Intel Macs and Windows; the toolchain fetched by
  rust-js, not installed by hand.
- **What stays**: the committed JS beside the Rust (ADR 0041),
  `js::directive!("use client")`, `js::export_default!`, and the binding
  crates' own form for the bindings rust-js ships (ADR 0373).
- **Measured against this**: each rule is a gap until a test of a
  TypeScript Next.js app, react.dev, holds it: install, one line, one file
  moved, every importer unchanged, `next build` with no toolchain.
- Vite is the same five rules, its plugin the one line; not decided here.
