# 0197. What's moved before anything can leave isn't its scope's to drop

Status: Accepted. Amends [0098](0098-destructors.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A value with a destructor that a scope owns is dropped as the scope ends,
by a `finally`, unless it was moved: its flag says (ADR 0098). One moved
first, before anything in the scope could panic or return, was flagged
all the same, as react.dev's `ExternalLink`, its children moved straight
into the JSX it returns, showed:

```js
let children$live = true;
try {
  children$live = false;
  return <a ...>{children}</a>;
} finally {
  if (children$live) dropC?.(children);
}
```

## Decision

**A value whose flag is cleared first in its scope, before anything that
can leave, and never set again, has no flag, and no `try` for it:**
`return <a ...>{children}</a>;` alone. Its drop can't run: nothing before
the move can leave, and nothing after it owns the value. (Amended: a `let`
that can't leave may come first, ADR 0301.)

## Why

- **It's exact, and the JS a person writes**: a test compiles a function
  that moves its value into what it returns, with neither flag nor `try`,
  and one that may panic first, which keeps both; the corpus's drops,
  which compare with native Rust's, are unchanged.
