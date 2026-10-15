# A Vite and React app, in Rust

`src/App.rs` is this app's component, written in Rust. rust-js compiles it
to `src/App.jsx`, the component you'd have written by hand, and Vite serves
that with Fast Refresh. Otherwise it's create-vite's React template (Vite 8,
`@vitejs/plugin-react`, React 19), with React Compiler and Tailwind CSS.

```bash
bun install      # or npm install
bun run dev          # or npm run dev: http://localhost:5173
bunx rust-js build   # or npx rust-js build, in another terminal: after you edit src/App.rs
bun run build        # or npm run build: dist/, once rust-js build --check says the JS is current
```

```
edit App.rs ─► rust-js build ─► App.jsx ─► React Compiler ─► Vite HMR ─► Fast Refresh keeps state
                                       └─► Tailwind reads App.jsx's classes ─► index.css updates in place
```

`vite` and `rust-js build` are two processes, each its own: Vite serves
the app as it always does, and `rust-js build` compiles the Rust once each
time you run it. A compile error is rustc's, in its terminal, and the page
keeps running the last version that compiled. The browser's source map
points at `App.rs`.

- **rust-js runs with the Rust release it's built with**, rustup's: it needs
  rustc's own library, and checks the app for `wasm32-unknown-unknown`. If
  it says that release isn't installed, install it as `bun create @rust-js`
  said to, and as it says: `rustup toolchain install <release> --profile
  minimal --target wasm32-unknown-unknown`, about 500 MB, once.
- **Bun or Node.js** installs the app and runs Vite. Node.js must be Vite 8's,
  `^20.19.0` or `>=22.12.0`, with Bun too if it's installed: `bun run dev`
  runs Vite on it.
- **`src/App.jsx` is committed**, as ReScript recommends for its JS: a diff
  shows what a change did to the output, and a checkout without rust-js
  still builds from it. Its source map, `App.jsx.map`, isn't.
- **React Compiler** memoizes the component rust-js writes, as it would one
  written by hand: `babel({ presets: [reactCompilerPreset()] })` in
  `vite.config.js`.
- **Tailwind CSS** reads `App.jsx` for class names, so `App.rs` uses its
  classes directly, and not the `.rs` files, so an edit of one before it's
  built doesn't reload the page. Write each class name whole: `if even {
  "text-emerald-500" } else { "text-sky-500" }`, not a string built from
  pieces.
- **The app is a Cargo package**, `Cargo.toml`, which `rust-js build` builds
  with Cargo, and your editor's rust-analyzer checks. Its crates, `react`,
  `webapi` and `js`, are npm packages, `@rust-js/react` and the others, named
  in `Cargo.toml` by version: `rust-js build` tells Cargo they're in
  `node_modules`, a patch in `.cargo/config.toml` it writes each time it runs. A binding of another
  library, `@rust-js-bindings/canvas-confetti`, is added the same way: its npm
  package, and a line in `Cargo.toml`. rust-analyzer checks it through
  rust-js, `rust-js-check` in `.vscode/settings.json`, which reads inside
  `jsx!`: what's wrong in JSX shows where it is, and what's used only there
  is used. Its own completion and hover still don't look inside `jsx!`.
- **The JS is laid out by your Prettier**, where it finds a config,
  `.prettierrc`, as `prettier --check` reads it, and rust-js's own layout
  where not. `Cargo.toml` can run your own tools on it, a linter say, in
  `[package.metadata.rust-js.hooks]`: what a linter says of `App.jsx` is said
  of the line of `App.rs` it came from.
