# 0845. An `Rc`'s declaration is what it points at

Status: Accepted. Extends [0196](0196-typescript-declarations.md), whose
`Rc` was `any`, to ADR 0023's and [0843](0843-a-weak-nothing-upgrades-is-the-value-itself.md)'s
uncounted `Rc` and `Weak`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. The declaration says
what the JS is: an uncounted `Rc` is the value it points at, and its
`Weak` that value or `null`.

## Context

React's context is an `Rc`, its `Provider` and its `Consumer`'s
`_context` `Weak`s (ADRs 0843, 0844). Its JS is upstream's, but its
declaration wasn't: every `Rc` and `Weak` was `any`,

```ts
export interface ReactContext<T> { /* .. */ Provider: any; Consumer?: any | null | undefined; }
export interface ReactConsumerType<T> { $$typeof: symbol; _context: any; }
export function createContext<T>(defaultValue: T): any;
```

where the port's `&'static` references had declared `ReactContext<T>`.
A type looser than the JS, `any`, is a port's gap.

## Decision

**An `Rc<T>` or `Arc<T>` whose counts nothing reads is declared `T`, as a
`Box<T>` is; its `Weak<T>` is `T | null`, `null` of a `Weak::new()`.** A
counted one, `{ value, strong, weak }`, stays `any`. The declarations ask
the one answer, `Counted::counts`, of what's counted.

```ts
export interface ReactContext<T> { /* .. */ Provider: ReactContext<T> | null; Consumer?: ReactConsumerType<T> | null | undefined; }
export interface ReactConsumerType<T> { $$typeof: symbol; _context: ReactContext<T> | null; }
export function createContext<T>(defaultValue: T): ReactContext<T>;
```

## Why

- **It's the JS's type**: the value an uncounted `Rc` is, and a `Weak`'s
  `null` where it points at nothing, which TypeScript must check for, as
  the Rust can't upgrade one either.

## Alternatives

- **A `Weak` as `T`, as upstream's Flow types `_context`**: a
  `Weak::new()`, `null`, would type-check as a context. Rejected.
- **Declare a counted `Rc` as `{ value: T; strong: number; weak: number
  }`**: right, but no consumer reads one yet; deferred.

## What holds it

- `test/declarations.test.ts`, "an Rc is declared as what it points at,
  and its Weak as that or null": a context's `Provider`, its maker's
  result, a counted `Rc`'s `any`, and TypeScript checking a module
  reading them.
- Mutations in `scripts/mutations/lower/declarations.ts`
  (`uncounted-rc-declared-any`, `weak-declared-never-null`).
