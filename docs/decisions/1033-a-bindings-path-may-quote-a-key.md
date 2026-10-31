# 1033. A binding's path may quote a key, which a minifier keeps

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): third, results change:
`window.$RT` and `window['$RT']` read the same property, but a minifier
that renames properties, Closure's ADVANCED mode, renames the first and
never the second, so after it only the quoted one is the property another
script set.

## Context

React DOM's Fizz runtime and the server's inline scripts share globals by
name: the inline scripts, compiled apart, set `$RT`, `$RB`, `$RV`, `$RM`,
`$RC`, `$RX`, `$RR` and `$RS` on `window`, and the external runtime reads
and sets them. Upstream writes each `window['$RT']`, on purpose:

```js
// Symbols that are referenced outside this module use dynamic accessor
// notation instead of dot notation to prevent Closure's advanced compilation
// mode from renaming.
window['$RT'] = performance.now();
```

A binding's path, `#[link_name = "window.$RT"]`, could only name each
property by its name, so the port printed `window.$RT = performance.now();`
(gap rd273): the same in JS as it is, but React builds these files with
Closure's ADVANCED mode (rd274), which may rename `$RT` to `a`, and the
runtime would then read a global the inline scripts never set.

## Decision

**A binding's path is JS's: the global it starts from, then each
property, `.name` by its name or `['key']` by a quoted key**, which prints
`window["$RT"]`. One form, the binding's, which says which property a
minifier must keep: `#[link_name = "window['$RT']"]` on a `static`, and
`get`, `set`, a call, `new` and `instanceof` of a function's path, all
read it the same way.

```rust
unsafe extern "Rust" {
    #[link_name = "window['$RT']"]
    safe static RT: Option<&'static js::Unknown>;
}

#[cfg_attr(rust_js, rust_js::link_name = "set window['$RT']")]
fn set_rt(time: f64) {
    let _ = time;
    unreachable!()
}
```

is `window["$RT"] = time;`, and `js::is_number(RN)` of one is `typeof
window["$RN"] === "number"`.

- `bindings::paths` owns a path's grammar: `first_name` (the global, which
  the names a module keeps clear of read, `window` of `window['$RT']`),
  `steps` and `read`, each a `Step::Named` member or a `Step::Quoted`
  index of its key. Each reader of a path, `global`, an import's rest,
  `js::is_defined`'s parts and a global's `set`, reads through it.
- **A path that isn't JS's is refused where it's declared**, naming what
  to write: a property by its name that isn't a JS name, `window.$R-T`, is
  quoted, `['$R-T']`, its one form, where it printed `window["$R-T"]`
  from the dotted form before; a key in double quotes, or one with a quote
  or a backslash in it, isn't a key. An import's path is checked past its
  export, `scheduler#*.unstable_now`'s `.unstable_now`.
- **A method's property is its name, never quoted**: a method is looked
  up on what it's given, `this.matchMedia`, whose name a minifier with the
  browser's externs keeps already; its `link_name = "['matchMedia']"` is
  refused, saying so.

The printer writes every string in double quotes, so upstream's
`window['$RT']` is `window["$RT"]`, which Closure reads the same.

## Alternatives

- **A window-global binding of its own**, a `#[rust_js::window_global]`
  attribute: a second syntax for a path's property, and only of `window`;
  the quoted key is JS's own, of any path.
- **Every `$`-named property quoted**: a guess from a name, where the
  binding is the one that knows another script reads it.
- **Keep `window.$RT`**: the same without a minifier, but wrong after
  Closure, the step React's build runs on these files.

## Consequences

- Held by `test/lowering-binding-paths.test.ts`: a quoted key read, set,
  asked `typeof` of and read through, with a local `window` beside it,
  run; and each malformed path refused. Mutations `quoted-key-named`,
  `path-first-name-whole`, `path-named-step-unchecked`,
  `path-key-quotes-unchecked` (`bindings/paths.ts`),
  `binding-path-unchecked`, `method-path-quoted`, `import-path-unchecked`
  (`bindings.ts`),
  `global-set-quoted-refused` (`binding_calls.ts`),
  `global-name-of-quoted-path` (`analysis/naming.ts`) and
  `asked-quoted-key-named` (`mir/calls.ts`).
- The port writes `#[link_name = "window['$RT']"]` and `"set
  window['$RT']"` for each global the inline scripts share.
