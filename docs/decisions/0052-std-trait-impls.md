# 0052. The crate's own `Default`, `From` and `Clone`, and the trait ABI kept

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and [0020](0020-structs-and-tuples.md).
(Amended: a derived impl here is std's derive's; another crate's derive's is
the crate's own code, ADR 0186.)

Case: C, A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0049 gave traits a JS shape: a dictionary per impl, dictionaries as
extra arguments to generic functions, and `{ value, impl }` for `dyn`.
Only the crate's own traits, plus `Copy` and a hand-written `Default`,
could be implemented. Every other std trait impl was an error, and
ordinary Rust uses a few of them all the time: `#[derive(Default)]`,
`impl From<f64> for Meters`, and a `Clone` that isn't just a copy.

`.clone()` was also wrong where it compiled. A clone of a `Vec` was the
same array, so after `let w = v.clone(); v.push(3);`, `w` had three items
too. A clone of a struct holding a `Vec` shared the `Vec` the same way.

Once JS code calls generic Rust, the shapes from 0049 are an API that JS
callers depend on. So before adding std traits, this ADR settles them.

## Decision

### The shapes 0049 chose are kept

| | JS | Rule |
|---|---|---|
| An impl's dictionary | `circleShape()` | Lazy and cached. Named after the type, then the trait, then the trait's arguments: `metersFromF64`. A reference is `ref`, or `refMut`, without its lifetime: `refFlagsBitOr` of `impl BitOr for &'a Flags`. Two impls whose names collide are an error. |
| A dictionary | `{ area, name }` | A plain object of the trait's methods. A std trait's has only its required methods: `{ clone }`, `{ default }`, `{ eq }` ([0053](0053-partial-eq.md)). |
| A generic function | `total(shapes, TShape)` | The dictionaries come after the value arguments, in the order the bounds are written, `where` clauses included. |
| A trait object | `{ value, impl }` | |
| A generic impl | `vecShape(TShape)` | Cached by its dictionary arguments, so the same arguments give the same object. |

### `Default`

- **A hand-written impl is a call:** `Config::default()` is `configDefault_default()`.
- **A derived impl is written in place:** a struct becomes an object of its
  fields' defaults, `{ size: 0, tags: [], mode: "Off" }`. An enum becomes
  its `#[default]` variant, read from the constructor that the derived body
  names.
- **In generic code it's the dictionary's `default`:** `TDefault.default()`.

### `From` and `Into`

- **An impl's `from` is a function named like the others:** `impl From<u32>
  for Meters` gives `metersFromU32_from`. The trait's argument is in the
  name because a type usually has more than one `From`.
- **`x.into()` is the same call** as the `from` it resolves to, and so is
  the conversion `?` makes of an error (ADR 0035).
- **`From` has no dictionaries.** A generic `T: From<U>` stays an error:
  there is nothing to call through yet.

### `Clone`

**A clone is the value itself unless something could tell the two apart.**
That happens in only two cases:

- One of the two can change in place:
  - a `Vec` of a type that something takes `&mut` of (`push`, `sort`,
    `v[i] = x` all do);
  - an array or a cell;
  - a struct whose fields get assigned (ADR 0020's rule).
- Cloning runs code: a hand-written `clone`, or a generic `T`'s, which
  might be hand-written.

Everything else is shared, as it is today. When a copy is needed:

- **A derived impl copies the parts that need it:**
  - a struct is `{ ...s, tags: s.tags.slice() }`;
  - an enum copies only the variants that hold such a part:
    `m.TAG === "List" ? { ...m, _0: m._0.slice() } : m`;
  - a `Vec` whose items need a copy is `items.map((item) => ..)`.
- **A hand-written impl is a call:** `trackedClone_clone(a)`. A derived impl
  calls it too, for a field that has one.
- **In generic code, `T: Clone` takes a dictionary:** `twice(x, TClone)`
  calls `TClone.clone(x)`. A `T: Copy` copies with its `Copy` dictionary.
- **An iterator's `cloned()` and `copied()` clone each item** that needs it.
- **What isn't supported is an error:** `clone_from`, and cloning a std type
  rust-js doesn't model.

## Why

- **The shapes from 0049 hold up.** A JS caller of a generic function sees
  its normal arguments first. Dictionaries are plain objects that JS can
  build, and impl names are predictable. Changing any of this later would
  break callers, so it's fixed here.
- **Clones stay cheap and readable.** Most clones are of values nothing
  changes, and those stay `const t = s`. The rule for `Vec` is the one ADR
  0020 uses for structs, applied per type across the crate.
- **A hand-written `Clone` or `Default` means what it says.** It runs,
  whether it's called directly, from a derived impl, or through a generic.

## Alternatives

- **Copy every clone.** `structuredClone` or a deep copy is simple and
  always correct, but every `.clone()` of a string or a read-only `Vec`
  would allocate, and the output would be full of copies.
- **Dictionaries for `From`.** `T: From<U>` would need a dictionary per
  argument type, and it's rare in the code rust-js compiles. It can come
  later without changing anything here.
- **Track `Vec` changes per variable, not per type.** That would be more
  precise, but it's the last-use analysis ADR 0020 already puts off.

## Consequences

- Every generic function with a `T: Clone` bound takes a `TClone` argument.
- A `Vec` clone is a real copy wherever its type is changed anywhere in the
  crate. The playground's `files.clone()` is `files.slice()`, because it
  pushes to other `Vec<String>`s.
- `examples/std_traits.rs` is checked against native Rust, including
  clones that would share if they weren't copied.

## Amended by

- [0289](0289-copy-bounds.md): A `Copy` bound takes a copy only where a copy isn't the value

## Amendment: a part changed through a `&mut` of it

**A struct, a tuple or an array whose part is changed through a `&mut`
of it is changed in place**, as one whose part is assigned is: text or a
number's `&mut` is its place, so `copy[0].0.push('?')` writes the part,
`copy[0][0] += "?"`, and `bump(&mut copy[0].count)` too. Its clone is a
copy, not the original shared, which the clone changed with it: a wrong
answer, `[Token("a?")]` for `[Token("a")]`. A `&mut` of a part that's an
object, a `Vec`'s, changes that object, not its holder.

## Amendment: an array changed through a slice

An array's type was changed only where its item was assigned, `a[i] =
v`, or it was replaced whole through a `&mut`; std's slice methods take
an `&mut` of it unsized to `&mut [T]` and change it there, as `iter_mut()`
and a function of `&mut [T]` do, which recorded nothing. So `let b = a;
a.sort();` shared one array, `fn sorted(mut a: [i32; 3])` sorted its
caller's, and a grid's rows, `[[0; 2]; 3]`, `new Array(3).fill([0, 0])`,
were one row changed through `grid[0].iter_mut()` (#125, #135). An array
something takes `&mut` of is changed now. A copy of an array made where
it's copied, a literal or a `.slice(..)`, of items that need none, is the
array itself, as nothing else holds it.

Consequence: an array of arrays a `for` takes the items of by value,
`for p in [[0, 0], [0, 1]]`, copies each where some `[i32; 2]` is changed
in the crate, `const p = value.slice()` in a `while (true)`, where it was
a `for (const p of ..)`: the items of a temporary array are its own, which
the loop's lowering doesn't yet know (`slice_patterns`).

Held by `test/corpus/arrays_changed_through_slices.rs` and the mutations
`array-borrowed-mut-unchanged` and `fresh-array-copied`.

## Amendment: a pattern's `ref mut` binding changes what it's in

An enum matched with a `ref mut` binding was recorded as changed, but a
tuple's or a struct's wasn't: `let (a, _) = &mut t; *a = 10;`, a binding
of default binding mode through a `&mut` subject, wrote `t[0]` where a
copy made before, `const o = t`, was the same array (#126); so did `if
let`, `match` and a `for` over `iter_mut()` of tuples. `ref_mut_holders`
records each object-like type a `ref mut` binding is a field of, at any
depth, an enum's variant's too, of every pattern, a `let` statement's
included: one walk, where the enum's were two arms of their own. Held by
`test/corpus/ref_mut_bindings_copied.rs` and the mutation
`ref-mut-holders-unseen`.

## Amendment: a `LinkedList`'s clone

Which std collections are a JS array was two lists: `is_vec_like`'s, of
`Vec`, `VecDeque`, `BinaryHeap` and `LinkedList`, and the analysis of
which are changed, of all but `LinkedList`, so a clone of one changed
after was the same array (#157). `VEC_LIKE` is the one list, which both
ask, and the architecture test holds it. Held by
`test/corpus/linked_list_clone.rs`.

## Amendment: rows changed through `as_flattened_mut`

An array something takes `&mut` of is changed; but `as_flattened_mut()`
of a grid, `[[u8; 2]; 3]`, takes the grid's and changes its rows, the
inner arrays, which weren't recorded, so `[[0; 2]; 3]` was one row three
times (#159). A call of it records each row's type as changed. Held by
`test/corpus/flattened_rows_apart.rs`.

## Amendment: an `Option` written through a `&mut` is its place

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a copy of what
nothing changes in place is the value itself, which nothing tells apart.

An enum something takes `&mut` of was changed in place, an `Option`
too: React's act captures its `result`, an `Option<&Unknown>`, by `&mut` in
the closure it catches, so every `Option<&Unknown>` of the crate was
changed, and ReactJSXElement's `children.to_vec()` a copy of each item,
`children.map((item) => item)`, where it was `children.slice()`. An
`Option`'s `Some(x)` is `x` (ADRs 0030, 0033), and a `&mut` of one is a
handle on its place: nothing of it is an object changed in place, a boxed
`Some(None)` neither, which a write replaces whole. **An enum is changed
through a `&mut` of it where it's object-like** (`object_like`, the one
answer the rules of `*r = v` and `ref mut` ask), and what its `Some` holds
is changed as itself, `p.x = 2` of `a[0].as_mut()`'s `Point`, still copied.

Held by `test/lowering-collections.test.ts`, "an option written through a
&mut is no object changed in place", run, and the mutation
`option-mut-changes-in-place`.
