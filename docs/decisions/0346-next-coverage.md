# 0346. The next crate binds all of Next.js, and coverage only grows

Status: Accepted. Extends [0314](0314-std-data-structures.md)'s ratchet to
the next crate, as [0102](0102-js-and-webapi.md) holds webapi's.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

The next crate grew by what react.dev's port needed: pages' `Link`,
`Image`, `Head`, `useRouter`, a few of `next/navigation`'s hooks, the data
fetching types, `next/document` lately. Nothing said how much of Next.js
that is, and a Rust app wanting `next/script`, `next/headers` or a route's
`NextResponse` found nothing. Next.js is as much rust-js's promise as React
is: the react crate follows `@types/react`, and the next crate follows
Next.js's own `.d.ts`.

## Decision

**The next crate binds every export of each public Next.js module, typed
as Next.js types it, and how many is measured against Next.js itself and
only grows.**

- **The measure**: `next/coverage.ts` reads each module an app
  imports (`next`, `next/app` to `next/web-vitals`, `next/font/google`,
  `next/font/local`, `next/offline`) from the installed Next.js's `.d.ts`,
  following its re-exports, and lists each export, `+ next/link#default`
  where the crate binds it, a value by its `link_name`, a constructor's
  `new next/server#NextRequest` and a static's too, a type by an item
  of its name, or a `pub use` of it, as is a value an item stands for, a class by its struct or
  a const object of strings, `RedirectType`, by its enum; `-` where it
  doesn't, under `# next/link 2 of 3`.
- **Left out**: build tooling (`next/babel`, `next/jest`), Next.js's
  internals (`next/client`, `next/constants`, `next/types`), and
  `next/root-params`, typed only once an app is built.
- **The ratchet**: `next/coverage.txt` is that list, which
  `test/next-coverage.test.ts` holds: an export bound before and unbound
  now fails, and one newly bound is blessed in (`BLESS=1`).
- **Each binding is tested** in a real `next build` in `test/next.test.ts`.

## Why

- **"All of Next.js" can be checked**: the list is Next.js's, so a new
  Next.js release's exports are counted where they appear.
- **It's the order of the work**: what's `-` is what's left.

## Consequences

- A type counts as bound by its name alone, wherever the crate defines
  it; whether it's typed as Next.js types it is each binding's test.
- `next/font/google` is 1942 functions, one a font: they'll be generated
  from its `.d.ts`, not written.
- Today: `next` 4 of 37, `next/navigation` 5 of 16, `next/document` 4 of
  6, `next/router` 4 of 5, `next/server` 0 of 14, `next/cache` 0 of 10,
  `next/dynamic` 0 of 12, `next/script` 0 of 5, `next/headers` 0 of 3.
