# 0438. An optional prop is given `Some(..)`, or left out

Status: Accepted. Extends [0213](0213-props-as-written.md) and
[0435](0435-rust-and-react-two-worlds.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): how a prop is written
changes nothing a program does; and "Idiomatic Rust in" (AGENTS.md).

## Context

A component's optional prop is an `Option<T>` field. `jsx!` leaves one
out as `None`, and takes a given one as its value, so it's written
`maxWidth={Some("450px")}` where TSX writes `maxWidth="450px"`. ADR 0435
made `jsx!` write JSX's names; this asks whether it writes JSX's values
too, a prop's value bare.

Rust can't take both a `T` and an `Option<T>` for one field and still
coerce the `T` to the field's type. Real props need both:

- **Coercion**, where the field's type is what the value is checked
  against: `onSubmit={Some(Rc::new(move || submit(false)))}` makes an
  `Rc<dyn Fn()>` of a closure's `Rc`, and `title={Some(&name)}` a `&str`
  of a `&String`.
- **Forwarding an `Option`**, as React code passes on a prop it was given:
  the playground's `FileTree` gives `onDelete={on_delete.clone()}`.

Each way to take a bare value loses one of them, which was tried:

- **Through `Into`**, as `impl Into<Option<T>>` takes it: forwarding
  works, and `width={100}` infers, but no coercion is made, so a boxed
  closure, a `&String` and `x.into()` are each rustc's error; six of the
  JSX tests and the playground stopped compiling.
- **Wrapped in `Some`**: coercions work, but an `Option` forwarded is
  `Some(Option<..>)`, rustc's error, and would need a second form.

## Decision

**An optional prop is given as Rust gives an `Option`, `Some(value)`, or
left out for `None`.** One form, the field's own type, which every
coercion and every forwarded `Option` works through:

```rust
jsx! {
    <Image src="/next.svg" alt="Next.js logo" width={Some(100)} height={Some(20)} priority={Some(true)} />
    <Editor state={state} onSubmit={Some(Rc::new(move || submit(false)))} />
    <FileItem name={name} onDelete={on_delete.clone()} />
}
```

- **Nothing else to write**: a prop left out is `None`, a binding's too,
  through its props' companion (ADR 0213), so `{..Default::default()}` is
  needed only to give a base of one's own. The next.js example, its
  bindings' docs and this decision's examples write none.
- **Not a limitation**: it's off the known limitations page, as it's what
  the program's types say, as written in any Rust.

## Why

- **It's Rust**: a field of `Option<T>` is given an `Option<T>`, as a
  struct literal, a builder or a function call is, so what's learned
  writing JSX is the Rust a server is written in.
- **One form that always works**: a value bare would be a second form, or
  break coercions, which a UI's handlers and borrowed text are made of.

## Consequences

- `jsx!`'s props read `Some(..)` where TSX's don't. JSX's own attributes,
  an element's (`className="c"`, `width={300}`), take their value bare, as
  their builders do; only a component's optional props are `Option`s.
