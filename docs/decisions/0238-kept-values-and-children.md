# 0238. A value kept for good is the value; children are kept as JS values

Status: Accepted.

Case: A, N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Challenges parses its children into challenges, which its
render and an effect both read, and its Navigation keeps a ref for each
challenge's button:

```js
Children.forEach(children, (child) => { .. content.push(child) .. });
const challengesNavRef = useRef(challenges.map(() => createRef()));
```

In Rust an effect takes what it reads for good, `'static`, and render reads
it too: one value both hold. `Vec::leak` makes that, a `&'static [T]`, which
rust-js refused. `children::for_each` gave each child for its call only,
`Child<'_>`, so none could be kept; `Child` wasn't `Copy`, though it holds
only references; and react had no `createRef`.

## Decision

**`Vec::leak` and `Box::leak` are the value itself; `for_each` gives each
child as a JS value kept as long as it's held, `Child<'static>`, which is
`Copy`; and `react::create_ref` is React's `createRef`.**

```rust
let mut kept = Vec::new();
children::for_each(&children, |child| kept.push(child));
let kept: &'static [Child<'static>] = Vec::leak(kept);
let refs = use_ref(kept.iter().map(|_| create_ref::<&HTMLButtonElement>()).collect::<Vec<_>>());
```

```js
const kept = [];
Children.forEach(children, (child) => {
  kept.push(child);
});
const refs = useRef(kept.map(() => createRef()));
```

- **JS frees nothing itself**, so what's kept for good is what's held: no
  copy, no wrapper.
- **`create_ref::<T>()` is a `RefObject<Option<T>>`**, `{ current: null }`,
  as @types/react's `RefObject<T | null>`.

## Why

- **It's the JS a person writes**, with one array both an effect and the
  render read.
- **It's tested**: the corpus's `leak` runs both leaks beside native Rust;
  a JSX test keeps a component's children from `Children.forEach`, each
  with a `createRef`, and renders them.

## Since

- **A `String` kept for good, `.leak()`, is the string**, as react.dev's
  Page gives Seo the image it makes, its props `'static`. Its `&mut str`
  was a `&str`, as nothing wrote a `str` in place; since ADR 0334 it's a
  cell of the string, `{ value: s }`, as any `&mut str` is, and read as
  a `&'static str` it's still the string. A compiler test leaks one;
  mutations leave `leak` unknown.
