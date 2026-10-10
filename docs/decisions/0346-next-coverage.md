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
  of its name, a `pub use` of it, or a namespace's by its `pub mod`,
  `MetadataRoute`, as is a value an item stands for, a class by its struct or
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

## Amendment: members

An export bound by its name may bind few of its members, as `ImageProps`
once had a handful of its fields. Each type's and class's own members are
listed too (2026-10-10), after it, `+ next/image#ImageProps.src`: an
interface's and what it extends of its file's, an object type's, of an
intersection's or a union's parts, `Omit`'s, `Pick`'s and `Partial`'s of
one, and a class's properties, methods and statics, of its text, its
private ones aside, and one named `_bfl`, as Next.js names an internal. One imported and exported again, `ImageProps` of
image-external, is read where it's declared. What a type has of React's,
an element's attributes, is React's, counted by its crate. Each is bound
where the Rust item of its name has it: a field by its JS name, not a
flattened one's struct's, React's among them; a method of its `impl`s by its link name, `get cookies`
a `cookies`; a static, `next/server#NextResponse.json`; an untagged enum's,
its payloads'; a type alias's, its type's. A member Next.js declares only
to throw, `NextRequest.page`, isn't bound, and stays `-`. A type a function
gives but no module exports, `AppRouterInstance`, isn't counted yet.
Today: 224 of 400.

## Amendment: a type's members are its declaring file's

Shapes were kept by name alone, so `next#Route`, `string & {}`, was given
another file's `Route` members, and next/legacy/image's `ImageProps`
next/image's. Each export now carries where it's declared, `path#name`,
following re-exports and imports, a default import's too, and its
members are that declaration's.

## Amendment: Next.js 16.4.0

The example, and so the coverage, is of Next.js 16.4.0: `NextConfig`'s
`deprecated`, its experiments' changes, a segment's `ParamMatching`, and
next/cache's `CacheEntry`, `CacheHandler`, `prefetch` and `navigation` are
bound. `CacheLifeProfiles`, an empty interface an app augments with its
profiles' names, has no Rust form, as the `Infer…` types haven't.

## Amendment: what's left to JavaScript

The app's config, next.config.js, and a deployment adapter, the module its
`adapterPath` names, stay JavaScript, the user's choice: `NextConfig`,
`NextAdapter` and `AdapterOutput` are `~`, their members too, counted apart
from what's bound and what's missing (ADR 0354).

## Amendment: what isn't bound by design, and a `Deref` outside the crate

What isn't bound by design is `x`, counted apart, each list with why:
TypeScript's own (the `Infer…` type operators, `CacheLifeProfiles`), what
Next.js declares only to throw, and its internals. A `Deref` to webapi's or
node's type, `NextRequest`'s `Request`, has that type's members too.
`ResolvedViewport`'s fields are getters, in place of `get []`. Today every
export and member counted is bound; a type a function gives that no module
exports, `AppRouterInstance`, still isn't counted.

## Amendment: what an export's function gives or takes

A type a module's function gives or takes that no module exports,
`useRouter`'s `AppRouterInstance` or `headers`' `ReadonlyHeaders`, is its
module's too, once, `AppRouterInstance (useRouter's)`, with its members:
one step from a function's signature. It found `ReadonlyHeaders`, now its
alias of `Headers`, and `experimental_gesturePush`, new in 16.4.
