# 0368. `util.promisify` of a callback function

Status: Accepted. Extends [0272](0272-node.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's page route finds its Markdown with Node's callback functions,
made ones that give a promise:

```js
const readdir = promisify(fs.readdir);
const stat = promisify(fs.stat);
// ..
return (await stat(res)).isDirectory() ? getFiles(res) : res;
```

@types/node types `promisify` by overloads, one per arity: of
`(arg1, callback: (err, result) => void) => void` it gives
`(arg1) => Promise<result>`. A Rust function of a generic argument, `A`,
fixes `A`'s lifetime once: `stat`, made outside a loop, can't take a path
the loop borrows each time round.

## Decision

**`util::promisify(f)` takes a function pointer and gives a
`Box<dyn Fn(..) -> Promise<T>>`; a callback function `promisify` is used
of, `fs::readdir`, is bound as a static of that pointer's type.**

- `Promisify` is implemented for `fn(A1.., Box<dyn FnOnce(Option<E>, T)>)`
  and `fn(A1.., Box<dyn FnOnce(Option<E>)>)`, of none to five arguments,
  as @types/node's overloads; and for `for<'a> fn(&'a P, ..)`, a borrowed
  first argument, a path, whose promised function takes any borrow of it.
- `fs::readdir` and `fs::stat` are `safe static`s of
  `for<'a> fn(&'a str, Box<dyn FnOnce(..)>)`: called as functions are,
  `fs::stat(path, callback)`, and given to `promisify` as the pointer they
  are. A function item's type is its own, which no impl names.
- A rejection is what `.await` of the promise throws (ADR 0029), as
  `await readdir(dir)` does.

## Why

- **The JS a person writes**: `const readdir = promisify(readdir)`,
  `await readdir(dir)`.
- **@types/node's types**, the overloads by arity, and its `fs.Stats`.

## Alternatives

- **`fs/promises`'s `readdir`**: the JS isn't the original's.
- **A generic `promisify<F: Fn(A, ..)>`**: one lifetime for every call of
  what it gives, so a borrow shorter than the function's own is refused.

## Consequences

- A callback function bound as a function item can't be promisified until
  it's bound as a static too; `readdir` and `stat` are.
- `test/bindings.test.ts` lists a directory through both, and holds the
  rejection of one that's missing.

## Since

2026-10-13.
