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
have. Then the app is one crate whose root lists every file; `dev` and
`build` become `rust-js-next`; a `Cargo.toml`, a patch step and editor
settings are written by hand. A Rust file that imports a TS one hand-writes
its binding, unchecked, from the crate root's directory. A TS file that
imports a Rust one sees `.d.ts` only if a flag says so, and snake_case
names unless another does.

## Decision

**A file's language is invisible to everything that imports it.** Moving a
file to Rust is writing `X.rs` and deleting `X.tsx`. Nothing else changes:
not its importers, TS or Rust, not the scripts, not the deploy.

### 1. Adding rust-js is one package and one line

```sh
npm install rust-js
```

```js
// next.config.js
const { withRustJs } = require('rust-js/next');
module.exports = withRustJs(nextConfig);
```

That is all. `next dev` and `next build` stay the commands; CI, Vercel and
teammates change nothing. The one package holds the compiler for the
machine (macOS, Linux and Windows), the runtime, and the bindings of React,
Next.js, the web platform and JavaScript. Nobody installs Rust by hand: the
first compile fetches the pinned toolchain into rust-js's cache.

The app's `Cargo.toml` is written the first time rust-js sees a `.rs` file,
its name the app's, and is the app's from then on, the place for a crate
from crates.io:

```toml
[package]
name = "react-dev"
edition = "2024"
```

### 2. Every `.rs` file is a module, as every `.ts` file is

No crate root to pick and no `mod` list to keep. The crate is the app's
source directory, `src/` where the app has one, as Next.js reads it, and
each file a module of it by its path: `src/components/ExternalLink.rs` is
`crate::components::external_link`. A `mod x;` naming a file is an error
that says so. A route file whose name isn't a Rust name, `[slug].rs`, is a
module only Next.js imports.

One file in, one file out: `ExternalLink.rs` compiles to
`ExternalLink.js` and `ExternalLink.d.ts` beside it, at the path the `.tsx`
had, its `mod x { .. }` blocks inside it.

### 3. Importers don't change

**A TS file importing a Rust one** reads its `.d.ts`, always written, and
the names JS gives: `pub fn format_date` is `formatDate`, as ADR 0376's one
naming rule reads a JS name back. `import {ExternalLink} from
'components/ExternalLink'` resolves, type-checks and renders as before.

**A Rust file importing a TS one** writes what it would for a Rust one:

```rust
use crate::components::external_link::ExternalLink;
```

rust-js binds each TS module a Rust file imports from TypeScript's own
types of it, the reverse of what `.d.ts` writes, and nobody writes a
binding of the app's code. A type Rust can't hold as TypeScript types it is
an error at the import, never a guess narrower or looser. When the TS
changes, rustc checks the Rust against it; when its file moves to Rust, its
Rust importers don't notice.

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
writes it fresh, `npx rust-js`.

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

After `npm install rust-js` and the one line in `next.config.js`, the
author writes `src/components/ExternalLink.rs`, deletes the `.tsx`, and
saves:

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
`.js` and `.d.ts`, the deleted `.tsx`, `Cargo.toml` and the line in
`next.config.js`.

## Consequences

- **What goes**, each a second way once this is delivered: `rust-js-next`
  and its scripts; the crate root and its `mod` list; the `declarations`
  key; `js::camel_case!()`; a hand-written binding of the app's own
  modules, and its root-relative path; the `rust-js-patch` step; the
  separate `@rust-js/*` packages an app installs.
- **What stays**: the committed JS beside the Rust (ADR 0041),
  `js::directive!("use client")`, `js::export_default!`, and the binding
  crates' own form for the bindings rust-js ships (ADR 0373).
- **Measured against this**: each rule is a gap until a test of a
  TypeScript Next.js app, react.dev, holds it: install, one line, one file
  moved, every importer unchanged, `next build` with no toolchain.
- Vite is the same five rules, its plugin the one line; not decided here.
