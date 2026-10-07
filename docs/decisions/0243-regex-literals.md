# 0243. A RegExp of a pattern as it's written is a literal

Status: Accepted. Extends [0102](0102-js-and-webapi.md).

## Context

react.dev's ErrorDecoder fills a message's `%s` in, and finds its links,
with regular expressions written as literals:

```js
return msg.replace(/%s/g, function () { .. });
```

The builtins crate makes one with `reg_exp::new(pattern, flags)`, which
rust-js wrote as JS's constructor, `new RegExp("%s", "g")`: the same
expression, but not as JS writes it, and with each `\` doubled, a
pattern's own escapes hard to read.

## Decision

**`reg_exp::new` of a pattern and flags written as they are is their
literal, `/%s/g`, when JS parses it as one.**

```rust
reg_exp::new(r"^/\d+$", "m")
```

```js
/^\/\d+$/m
```

- **A `/` that would end it is escaped**, `\/`, but in a class, `[/]`,
  where it doesn't: what JS's `source` has, the same either way.
- **An empty pattern is `/(?:)/`**, its `source`: `//` is a comment.
- **One that JS doesn't parse is made as it runs**, `new RegExp("(")`, to
  throw then, not as the module is read; one with a line break too, which
  no literal holds. oxc's parser of regular expressions tells.

## Why

- **It's the JS a person writes**, its pattern read as it's written.
- **It runs the same**: a literal is a new `RegExp` each time it's
  evaluated, as the constructor makes one.
- **It's tested**: a compiler test makes a literal, one with a `/` in and
  out of a class, an empty one, and one with a line break and one JS
  can't parse made as they run; mutations break each case.
