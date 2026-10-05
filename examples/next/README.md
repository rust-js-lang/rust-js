# A Next.js app, in Rust

`app/page.rs` is this app's page, written in Rust. rust-js compiles it to
`app/page.jsx`, the route you'd have written by hand, and Next.js serves
that as it would any route. Otherwise it's create-next-app's App Router
template (Next.js 16, React 19, Tailwind CSS), in JavaScript.

```bash
bun install      # or npm install
bun run dev      # or npm run dev: http://localhost:3000; edit app/page.rs and save
bun run build    # or npm run build: .next/
```

```
save page.rs ─► rust-js-next: cargo check with rust-js ─► page.jsx ─► Turbopack ─► Fast Refresh keeps state
```

`rust-js-next dev` and `rust-js-next build` are `next dev` and `next build`,
the app's Rust compiled first, and again on each save. A compile error is
rustc's, in the terminal, and the page keeps running the last version that
compiled; `rust-js-next build` stops at one.

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
- **The app is a Cargo package**, `Cargo.toml`, which `rust-js-next` builds
  with Cargo, and your editor's rust-analyzer checks. Its crates, `next`,
  `react` and `js`, are npm packages, named in `Cargo.toml` by version:
  `rust-js-patch`, the app's `postinstall`, tells Cargo they're in
  `node_modules`. rust-analyzer doesn't look inside `jsx!`, so what's used
  only in JSX, `Image`, shows as unused there.
- **Cargo builds into `node_modules/.cache/rust-js/target`**, which Next.js
  doesn't watch, where a `target/` in the app would be many changes to it on
  each save. Give rust-analyzer the same, `rust-analyzer.cargo.targetDir`.

## Learn more

- [Next.js Documentation](https://nextjs.org/docs): Next.js's features and API.
- [Learn Next.js](https://nextjs.org/learn): an interactive Next.js tutorial.
