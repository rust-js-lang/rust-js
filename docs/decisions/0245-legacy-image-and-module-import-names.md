# 0245. next/legacy/image, and a module's imports named by its own

Status: Accepted. Extends [0192](0192-next.md).

## Context

react.dev's TeamMember shows each member's photo with Next.js's older
image, which fills its parent and covers it:

```tsx
import Image from 'next/legacy/image';
<Image src={withBasePath(photo)} layout="fill" objectFit="cover" alt={name} />
```

The next crate had `next/image` only. And where one module of a crate
imports `next/image`'s `Image` and another `next/legacy/image`'s, rust-js
named the second `Image$1`: every module named every import of the
crate, in one order, so the first of a name had it everywhere.

## Decision

**The next crate has `next::legacy::image::Image`, typed as Next.js's
`next/legacy/image`; and each module names the imports it uses before
the crate's others.**

```rust
use next::legacy::image::Image;
jsx! { <Image src={src} layout={Some("fill")} objectFit={Some("cover")} alt={Some(name)} {..Default::default()} /> }
```

```jsx
import Image from "next/legacy/image";
<Image src={src} layout="fill" objectFit="cover" alt={name} />
```

- **Its props are Next.js's**, each optional but `src`, `None` unless
  given, as `next/image`'s are (ADR 0192).
- **A module's own imports are named first**: one it doesn't use is named
  after them, and never written.

## Why

- **It's the JS a person writes**: each module imports `Image` by its name.
- **It's tested**: the Next.js build test renders a `next/legacy/image`
  that fills its parent and covers it, in a route beside one that imports
  `next/image`; a mutation names the crate's other imports first.
