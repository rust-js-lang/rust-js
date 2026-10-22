# 0600. The guide is the site on GitHub Pages

Status: Accepted. Replaces the playground's deployment to GitHub Pages
([0032](0032-dogfooding-the-playground.md), [0045](0045-playground-on-vite.md));
the playground itself stays, built and tested as it was.

Case: none ([0262](0262-when-rust-and-js-disagree.md)): no program's
behavior is touched; this is the project's site.

## Context

GitHub Pages served the in-browser playground, rust-js and rustc's front
end compiled to Wasm. Someone arriving at `rust-js-lang.github.io/rust-js`
met an editor, while the guide, `docs/guide`, how to write a JavaScript app
in rust-js, was Markdown read on GitHub: no search, no navigation between
pages beyond each page's links, and its Rust and JavaScript side by side
only as GitHub renders them.

## Decision

**Pages serves the guide, built as a static site:** each `docs/guide` page
an HTML page, `README.md` the index, with one stylesheet and one script.

- **`guide-site/`** builds it: `markdown.ts` renders the guide's Markdown,
  what its pages use and no more (headings with GitHub's anchor ids, so a
  page's own `#links` land; fences; tables; lists whose items hold
  paragraphs and fences; code spans, links, bold, italics); `highlight.ts`
  highlights each fence at build time, Rust, JS, JSX, TS, TOML, JSON and
  shell; `build.ts` lays each page out, writes `search.json`, and copies
  `assets/` and the logo, `docs/logo`.
- **The nav is the guide's README**, its one index: each `##` a section,
  each listed link a page, in order; a page it doesn't list goes under
  "More". "Previous" and "Next" follow that order.
- **A link to a guide page is to its `.html`;** one out of the guide, to a
  decision or the architecture, is to it on GitHub. A link that leads
  nowhere fails the build, and `test/guide-site.test.ts`.
- **Each page has** the nav, the page, "On this page" of its `##` and `###`,
  a copy button on each block, search of every page's sections (`/` or
  ⌘K), a light and a dark theme, and a layout for small screens.
- **`deploy-guide.yml`** builds it and deploys it, by hand as every
  workflow is (`bun run deploy`); `bun run guide-site` builds it into
  `target/guide-site`. It needs no Rust toolchain.

## Why

- **The guide is what someone new needs first**, and the one place that
  says what to write (its README): the site makes it what they meet.
- **No dependency.** The guide's Markdown is a small, regular subset, and
  its fences what rust-js writes; a renderer and a highlighter of it are a
  few hundred lines, owned here, with nothing to install or keep updated.
- **The guide stays Markdown in the repository**, read on GitHub as before,
  and checked as before (`bun run docs`): the site is made of it, never
  edited apart.

## Alternatives

- **A site generator** (VitePress, Docusaurus, mdBook): a framework and its
  dependencies for 29 pages, and its conventions over the guide's README
  as the index.
- **Keep the playground on Pages, the guide elsewhere:** two sites, or a
  playground first for someone who hasn't read how to write rust-js.

## Consequences

- The playground isn't deployed. `bun run site` and `bun run preview` still
  build and serve it locally, and the check still tests it. `bun run ship`,
  which published a prebuilt `rust-js.wasm` for its deployment, is gone;
  `wasm/prebuilt.sh` is unused.
- A guide page's Markdown beyond the subset renders as text: a new form
  needs the renderer extended, with a case in `test/guide-site.test.ts`.
