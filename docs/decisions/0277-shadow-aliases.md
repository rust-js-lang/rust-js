# 0277. A variable shadowed by its own value is the variable

Status: Accepted. Amended: another's variable only where nothing runs
before it's read (below).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust shadows a variable with what's made of it, and every local of a JS
function has a name of its own, so react.dev's errors page read:

```rust
let Type = ElementType::from_unknown(Type);
jsx! { <Type key={key} {...props} /> }
```

```js
const Type$1 = Type;
return <Type$1 key={key} {...props} />;
```

and `let content = content.clone();` before a closure was `const content$1
= content;`. Its original has `<Type ..>` and `content`.

## Decision

**`const n$1 = n;`, a variable shadowed by its own value, is `n`**: each
read of `n$1` reads `n`, and the `const` is gone. Only where `n` holds that
value wherever `n$1` is read: nothing sets `n` after it, nor in a closure,
nor in a loop it's in, where a closure of each turn keeps its own. One of
another name, `let kept = text;`, keeps the name written.

## Why

- **No program can tell**: the two hold one value, the same object or the
  same string, wherever the new one is read.
- **It's the JS a person writes**, and the name the Rust has.
- **It's tested**: a compiler test shadows a captured string and a number
  set before, and keeps one of another name and one a loop sets again,
  whose closures each read their own turn's; mutations alias each.

## Since

- **A closure's own parameter shadowed in it is the parameter too**,
  `(total, item) => total + item.length` of `let item = item.as_str();`,
  as react.dev's createFileMap casts each snippet it's given. Only its
  parameters: nothing outside the closure sets them.

## Amendment: another's variable only where nothing runs before it's read

A local named as a thread-local's module variable, `let count =
COUNT.get(); bump(); count`, was `const count$1 = count; bump(); return
count$1;` until this rule took `count$1` for `count`: nothing in the
function set `count` after it, but `bump()` did, so the function returned
6 where Rust returns 5, a wrong answer. A function's own variables only it
and its closures set, which the rule sees; a module's, or an import, any
code that runs.

**A `const n$1 = n;` of a name the function doesn't declare is `n` only
where each read of `n$1`, in JS's order, is before anything that may run
code**: a call, `new`, an `await`, a JSX element, a binding's getter or
setter. A call's callee and arguments are read before it runs, `if
(holder != null) { bump(holder); }`; a branch's way is its own; a loop, a
`try` or a `switch` that reads it runs nothing. Else it stays its copy,
`const count$1 = count; bump(); return count$1;`, and so does one read
past a binding's property, which a getter may answer, `const saved$1 =
saved; const now = button.onclick;`. The function's own names, its
parameters and what its blocks declare, a `catch`'s, a loop's and a
function's too, are as before; so are a closure's.

The rule is `src/prepare/shadows.rs`'s now, out of `prepare.rs`.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)):
Rust's answer, preserved. Tested by `test/corpus/shadow_of_module_variable.rs`
against native Rust; the playground's `setTheme` and the bindings test's
`same` keep their copy past a binding's method and getter. Its mutations
are `shadow-of-outer-variable-aliased`, `outer-shadow-read-past-a-call` and
`outer-shadow-read-before-a-call-copied`.
