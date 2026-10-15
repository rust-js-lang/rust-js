# A Next.js app, in Rust

`app/page.rs` is this app's page, written in Rust. rust-js compiles it to
`app/page.jsx`, the route you'd have written by hand, and Next.js serves
that as it would any route. Otherwise it's create-next-app's App Router
template (Next.js 16, React 19, Tailwind CSS), in JavaScript.

```bash
bun install          # or npm install
bun run dev          # or npm run dev: http://localhost:3000
bunx rust-js build   # or npx rust-js build, in another terminal: after you edit app/page.rs
bun run build        # or npm run build: .next/, once rust-js build --check says the JS is current
```

```
edit page.rs ─► rust-js build: cargo check with rust-js ─► page.jsx ─► next dev's Turbopack ─► Fast Refresh keeps state
```

Two processes, each its own: `next dev`, as Next.js runs it, and `rust-js
build`, which compiles the app's Rust once, each time you run it, to
`page.jsx` and `page.d.ts` beside it. Next.js sees the new JS as any
edit, and Fast Refresh keeps the page's state. A compile error is rustc's,
in `rust-js build`'s terminal, and the page keeps the JS that last
compiled. `bun run build` checks first that what's committed is what the
Rust compiles to, `rust-js build --check`, and stops if it isn't.

- **A Rust route is a page as Next.js finds one**: `app/about/page.rs`, a
  module of the crate, `mod about { pub mod page; }` in `app/page.rs`, is
  `app/about/page.jsx`. Its page is its default export,
  `js::export_default!(About);`.
- **A component is a Server Component**, as a JS one is, unless its module
  says `js::directive!("use client");`, as a component with state or events
  must: `"use client";` at the top of its JS.
- **Next.js's components and hooks are the `next` crate's**: `Image`, `Link`,
  and `next/navigation`'s `use_router`, `use_pathname` and
  `use_search_params`. A component's optional props are `None` unless given:
  `<Link href="/about" {..Default::default()}>{"About"}</Link>`.
- **`app/page.jsx` is committed**, as ReScript recommends for its JS: a diff
  shows what a change did to the output. Its source map isn't.
- **rust-js runs with the Rust release it's built with**, rustup's: if it
  says that release isn't installed, install it as `bun create @rust-js`
  said to: `rustup toolchain install <release> --profile minimal --target
  wasm32-unknown-unknown`, about 500 MB, once.
- **The app is a Cargo package**, `Cargo.toml`, which `rust-js build` builds
  with Cargo, and your editor's rust-analyzer checks. Its crates, `next`,
  `react` and `js`, are npm packages, named in `Cargo.toml` by version:
  `rust-js build` tells Cargo they're in `node_modules`, a patch in
  `.cargo/config.toml` it writes each time it runs. rust-analyzer checks it through rust-js, `rust-js-check`
  in `.vscode/settings.json`, which reads inside `jsx!`: what's wrong in
  JSX shows where it is, and what's used only there, `Image`, is used. Its
  own completion and hover still don't look inside `jsx!`.
- **Cargo builds into `node_modules/.cache/rust-js/target`**, which Next.js
  doesn't watch, where a `target/` in the app would be many changes to it on
  each save. Give rust-analyzer the same, `rust-analyzer.cargo.targetDir`.

## Learn more

- [Next.js Documentation](https://nextjs.org/docs): Next.js's features and API.
- [Learn Next.js](https://nextjs.org/learn): an interactive Next.js tutorial.
