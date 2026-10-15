# 0380. The app's config compiles its Rust: `withRustJs(nextConfig)`

Status: Superseded by [0412](0412-rust-js-build-is-its-own-process.md):
an app's Rust is compiled by `rust-js build`, its own process beside
Next.js. Amended [0192](0192-next.md); delivered rule 1 of
[0378](0378-one-file-at-a-time.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how a host starts
the compiler, not what a program means.

## Context

`rust-js-next dev` and `rust-js-next build` ran in place of `next dev` and
`next build`: an app's scripts changed, and an app that starts Next.js
another way couldn't use them. react.dev's `dev` is `next-remote-watch`,
which starts Next.js itself, `next({ dev: true })`; a custom server does
the same; and yarn 1 gives a linked package's command no
`node_modules/.bin` entry.

Whatever starts Next.js loads the app's config first, `next.config.js`,
and says why, the phase. Measured with Next.js 16: `next dev` loads it
twice, both in its server's process, and `next build` twice, in its own;
the command's parent loads it never.

## Decision

**An app's config compiles its Rust: `withRustJs(nextConfig)`, from
`@rust-js/next-plugin`, by the phase Next.js loads it in.**

```js
// next.config.js
const {withRustJs} = require('@rust-js/next-plugin');
module.exports = withRustJs(nextConfig);
```

- **It changes nothing of the config**: Next.js gets `nextConfig`, an
  object or Next.js's function of the phase, as it is. The JS it serves
  is the JS on disk, beside the Rust, so no loader, alias or webpack
  function of the app's is touched.
- **Developing** (`next dev`, a dev server any way started): the crate is
  compiled before the config is given, then again on each save, on through
  a failure, as before (ADR 0192, 0379).
- **Building** (`next build`): compiled once before Next.js builds; a
  compile error is rustc's, then Next.js's "Failed to load next.config",
  and no build. Without a compiler, the committed JS is built, as before.
  ADR 0378's rule 4, a build that checks the committed JS and compiles
  nothing, isn't this yet.
- **Any other phase**, `next start`, a production server: nothing.
- **Once a process**, however often Next.js loads the config: each phase's
  work is one promise, which each load awaits.
- The phases are Next.js's names, read from the app's `next/constants`.
- The app is the directory Next.js runs in.

`rust-js-next` has one command left, `compile`: the JS alone, for what
reads it before Next.js runs. Its `dev` and `build` are gone, a second way
to do what the config does; asked for, its usage names `withRustJs`.

## Consequences

- `examples/next`'s scripts are `next dev` and `next build`, and
  `@rust-js/create`'s Next.js template's with them.
- The dev server's watchers are Next.js's process's, and stop with it: no
  child to stop, nor a signal to forward.
