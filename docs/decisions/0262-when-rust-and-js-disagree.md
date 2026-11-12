# 0262. When Rust and JS disagree, what the program can observe decides

Status: Accepted. Spells out the rule at the top of the [design docs](../README.md).

## Context

The design docs open with one rule, borrowed from ReScript:

> Where the language and JS disagree, pick the JS behavior that keeps the
> output small and readable, and write that choice down.

Taken alone, it would allow any disagreement for smaller output. The
decisions since haven't read it that way: each asked whether the program
could tell the difference, and some kept Rust's meaning at a cost in output,
`>>> 0` after a `u32`'s sum, `$byteLen(s)` for a `len()`. That question was
never written down, so each decision asked it anew.

## Decision

**What the program can observe decides, in four cases:**

```
Can the program observe the difference?
│
├─ No ─────────────────────────────► A: JS's, and the ADR says why
│
├─ Yes, but only where it's rare ──► B: JS's, and the ADR's Consequences
│  and does no harm                     say where
│
├─ Yes, and results would change ──► C: Rust's, paid for in output
│
└─ Yes, and Rust's can't be kept ──► D: a compile error, never silent drift

No disagreement at all, tooling, bindings' shape, the output's layout: N
```

- **A, not observable: JS's.** JS runs a module on one thread, so a
  `thread_local!` is a module's `const` (ADR 0037). A fresh object of
  constants is the same made before or after what follows (ADR 0261).
- **B, observable where it's rare and harmless: JS's, written down.** A
  thread-local is made when its module loads, not at first use: only an
  initializer with effects can tell (ADR 0037's Consequences).
- **C, results would change: Rust's.** A `u32`'s arithmetic wraps, `x >>> 0`
  (ADR 0011). A string's `len()` counts UTF-8 bytes, `$byteLen(s)`, where
  JS's `length` would agree for ASCII and quietly disagree otherwise
  (ADR 0138).
- **D, Rust's can't be kept: an error.** `rust-js does not support .. yet`,
  and no output (ADR 0006). A later decision may move a case up, as ADR
  0138 did string byte counts, an error under ADR 0034. Where only running
  can tell, as an init that uses its own `LazyLock`, which deadlocks in
  Rust (ADR 0318), the runtime throws `rust-js does not support ..` there:
  never a panic that looks like std's but isn't what Rust does.

The program's meaning is rustc's: rust-js never makes Rust that's wrong
work in JS. Two closures, `Box::new(move |_| close())` given to
`addEventListener` and again to `removeEventListener`, are two functions,
as in Rust, though printing both as `close` would remove the listener.
react.dev's TopNav passes the same closure instead.

## Why

- **Every decision since has asked this**; written down, the next one
  starts from it.
- **It keeps both halves of the north star**: JS that looks hand-written
  where Rust can't tell, and Rust's answers where it can.

## Alternatives

- **The rule alone**: smaller output wherever JS and Rust disagree. A
  program's answers would change silently, `"héllo".len()` 5, not 6.
- **Rust's meaning everywhere**: every thread-local lazy, every object made
  in Rust's order. The output would be std's machinery, not the JS a person
  writes.

## Consequences

- **Each decision says which case it's in**, on a `Case:` line under its
  status, every case its parts are in, the first the main one; N where it
  decides no disagreement, tooling, bindings' shape or the output's
  layout. An unobservable difference says why the program can't tell; a
  rare one lists where it can, under Consequences.
- **A once-callback passed on is the function it calls**, the second
  case's, which the user decided (2026-10-26): a closure given to a
  binding's `FnOnce` callback that only passes its call on to a function
  of the crate's or of a library rust-js compiled (ADR 0605, amended), or
  a closure, `p.then(|_| ping(), |_| ping())`, is
  `p.then(ping, ping)`. Only the callback's identity tells it from a fresh
  closure's, which JS doesn't compare of a callback it calls once. Listed
  in the guide's differences (`semantics.md`, 27), below. Widened, the
  user's decision (2026-10-30, gaps rd196, rd212, ADR 0605, amended): a
  closure that ignores what it's given and calls, with nothing, a
  function that reads no `this` and nothing it's given more, the crate's
  or one bound, is that function wherever it's given, a listener's too,
  `addEventListener("load", ping)`. Its `.length` and its identity tell
  them apart: a JS caller of an exported value sees one function where
  Rust made a closure each time. A forwarding closure is the function
  itself, so adding it twice to one target and event makes one
  listener, where two closures were two: the user accepted this as part
  of the exception (2026-10-31), and so two `Box<dyn Fn>`s of one that
  `ptr::eq` compares are one; not an `Rc`'s or an `Arc`'s, whose
  allocation `Rc::ptr_eq` reads, which stays a function of its own. And
  a closure that passes all it's given, in order, to a namespace's
  function through a value is that function (the user's eta rule, ADR
  0605, amended), and so is one to a namespace's method by its name,
  `unstable_flushSync: flushSyncFromReconciler` (the user's decision,
  2026-11-01, ADR 1037, amended), only there: a dictionary's or a class's
  method reads its `this`. A closure written in place for removal
  is refused, as it removes nothing (ADR 1021, amended). And a function
  that only passes its parameters on to a function at one instantiation
  is that function, re-exported by its name (the user's decision,
  2026-11-02, gap rd575, ADR 1064): a JS caller sees the function itself,
  `===` it, its `name` and `length` the function's. Listed in the
  guide's differences (`semantics.md`, 45).
  And a dictionary's entry that passes on its first parameters, in
  order, to its impl's method, which declares just those as its owner
  says (`declared_arity`: its inputs, a writer's `options`, its evidence;
  none of a rest parameter's), `fromIter: (arg0, TIntoIterInPlace) =>
  bagFromIteratorU32_fromIter(arg0)` of a fact its trait carries and the
  impl doesn't read, is the method, `fromIter:
  bagFromIteratorU32_fromIter` (#801, assigned for the user by m6-react,
  2026-11-06). Case none for every caller rust-js writes, which calls
  the entry and reads nothing else of it: what it gives past them the
  method never names, as the entry didn't pass it on, and a method reads
  no `this`. A JS reader of an exported dictionary would see the
  method's `length` and identity, a forwarding closure's difference.
  Listed in the guide's differences (`semantics.md`, 27).
- **A binding getter's function unwrapped where it's called is the read**,
  the user's decision (2026-11-02, gap rd519, ADR 1066):
  `console::create_task().unwrap()(name)` is `console.createTask(name)`
  and `function::bind(console::create_task().unwrap(), ..)`
  `console.createTask.bind(console, ..)`. Of a host without it, JS's call
  throws a `TypeError` where Rust's `unwrap()` panics: a throw at the same
  place either way, its message and type the difference. Listed in the
  guide's differences (`semantics.md`, 46).
- **A `RegExp`'s `test` and `replace` take text, as TypeScript types
  them**, the second case's, which the user decided (2026-10-28, gap rd33):
  React's warnValidStyle tests `badStyleValueWithSemicolonPattern.test(value)`
  of any style value, and a port converts it first,
  `pattern.test(&js::string(value))`, `String(value)`. Only a symbol, whose
  `String` is its name where the regular expression's own conversion
  throws, and an object whose `replace` isn't String's tell them apart,
  neither a style value React is given. No binding takes any value: one
  looser than TypeScript would be a second form. Listed in the guide's
  differences (`semantics.md`, 28).
- **A method a value may lack is asked by its type,
  `js::is_function((selection, Selection::extend))`, `typeof selection.extend
  === "function"`**, the second case's, which the user decided
  (2026-10-28, gap opt1, ADR 0942, amended), where upstream tests its
  truth, `!selection.extend`: only an `extend` that's truthy and no
  function tells them apart. Listed in the guide's differences
  (`semantics.md`, 29).
- **A `pub` thread-local's initializer runs as its module loads**, the
  second case's, which the user decided (2026-10-28, gap rd51, ADR 1006):
  React DOM's `export const ANIMATION_END =
  getVendorPrefixedEventName('animationend')` is made where upstream makes
  it, so JS's importers read its value. What the initializer prints, a
  panic in it, and what it reads and changes are where its module loads,
  where Rust has them at its first use. One whose module's load can't make
  it before what it reads, an import cycle's or one reading itself, is
  refused. A private one is made as its module loads too (2026-10-29, gap
  rd120, ADR 1006, amended), React DOM's `const isScrollEndEventSupported =
  isEventSupported('scrollend')`, and where its module's load can't make
  it, where it's first used. Narrowed (2026-11-04, #554, ADR 1006,
  amended): one whose initializer prints, may panic, reads state a
  function that may run later sets, or changes state one reads is made
  where it's first used, a `pub` one refused; what bindings do, std's own
  panics, and what such initializers share among themselves alone stay at
  the load. Listed in the guide's differences (`semantics.md`, 30).
- **A package's default entry is its root**, the second case's, which the
  user decided (2026-10-28, gap rd55, ADR 1008, amended): React DOM's
  `import {Children} from 'react'` is of `react`'s `"default":
  "./index.js"`, ReactClient's; a host that resolves its `react-server`
  condition gets ReactServer's, another value, as upstream's import does.
  Only a host that sets a condition of the package's tells them apart.
  Listed in the guide's differences (`semantics.md`, 31).
- **A closure as a `fn` that only calls one host function is that
  function**, the second case's, which the user decided (2026-10-30, gap
  rd158, ADR 0605, amended): React DOM's `scheduleMicrotask` and
  `localRequestAnimationFrame` hold `setTimeout` itself where the port
  writes `|callback| { js::set_timeout(callback, None); }`. Rust's calls of
  the `fn` give it what the closure takes and read nothing it gives; a JS
  caller of an exported value tells them apart in two ways, both kept by
  the user's decision (2026-10-30): it sees what the host function gives,
  setTimeout's id where the closure gave `undefined`; and it may give it
  more than the closure takes, setTimeout's delay, which the host
  function reads. Listed in the guide's differences (`semantics.md`, 32).
- **A copy given where a borrow can't be held across a JS call**, the
  second case's, which the user decided (2026-10-31, gap rd246): React's
  startViewTransition and startGestureTransition give the browser the
  transition's shared types array, `{update, types: transitionTypes}`.
  The port can't hold the `RefCell`'s borrow of it across the call (ADR
  0863), rightly: a startViewTransition may call `update` at once, and
  the commit it runs may borrow the types. So it gives a copy,
  `transition_types.as_ref().map(|types| types.borrow().clone())`,
  `transitionTypes?.slice()`. Only the array's identity differs, the same
  strings in a new array. Only a replacement of startViewTransition that
  keeps the array and compares it, or watches it change, tells them
  apart; the browser copies `types` into the ViewTransition's set. No
  compiler change: it's the Rust the program writes. Held by
  `test/webapi-bindings.test.ts`'s "startViewTransition given a copy of
  the transition types". Listed in the guide's differences
  (`semantics.md`, 33).
- **A trailing `None` given a function value is no argument**, the
  second case's, which the user decided (2026-10-31, gap rd271, ADR 0443,
  amended): React's `callback()`, where the port writes `callback(None)`
  of a `dyn Fn(Option<..>)`, as a binding's trailing `None` is left out.
  A `dyn Fn`'s, a pointer's, a closure's or one `function::bind` makes.
  Only a JS function that counts its arguments, `arguments.length` or a
  rest parameter, tells `cb()` from `cb(undefined)`; no Rust function
  reads either. One that reads them so gives what it gives of what it's
  given: `Math.max` given from JS as a `dyn Fn(f64, Option<f64>)` and
  called `f(1.0, None)` gives `1`, `Math.max(1)`, where it gave `NaN`. A
  pointer the crate makes of a binding's function, whose parameter isn't
  optional, is still given its `undefined`. Held by the corpus's `option_arguments` and
  `test/js-functions.test.ts`'s "a trailing None given a function value
  is no argument". Listed in the guide's differences (`semantics.md`, 34).
- **A field's function called with arguments that need statements is
  called through its object**, the second case's, which the user decided
  (2026-10-31, gap rd285, ADR 1034): React DOM's Float,
  `(REACT_DOM_SHARED_INTERNALS.d().preload)(href, .., options read
  apart)`, is `const d = ReactDOMSharedInternals.d; ..; d.L(href, ..)`, as
  upstream writes it, its `this` `d` as the plain call's is. Its `L` is
  read after the argument statements, where Rust reads it first: only a
  getter that answers `L`, of a struct JS made, with effects that meet
  theirs tells. Limited to a field Rust can't change while the body runs,
  of what a `&T` parameter or a `&'static T` a call gives is to, no cell;
  one Rust can change between is read where Rust reads it. Held by
  `test/lowering-held-receivers.test.ts`. Listed in the guide's
  differences (`semantics.md`, 35).
- **`js::is_function` of a path guards each part before the last**, the
  second case's, which the user accepted (2026-10-31, gap rd320): ADR 0942's
  `typeof performance !== "undefined" && typeof
  performance.getEntriesByType === "function"` gives `false` on a host
  without `performance`, as Rust is told, where upstream's bare `typeof
  performance.getEntriesByType === "function"`, React DOM's
  estimateBandwidth's, throws a `ReferenceError` as its module or call
  runs. Only such a host tells them apart, and there the port goes on
  where upstream would stop. Listed in the guide's differences
  (`semantics.md`, 36) and global values (`interop/global-scope.md`).
- **A stream's write marked as running none of the crate's code**, the
  second case's, which the user decided (2026-11-01, React DOM's Fizz
  config): its stream config's `writeChunk(destination, chunk)` and
  `writeChunkAndReturn` are bindings marked `#[rust_js::runs_nothing]`
  (ADR 0867), though each calls the destination's `write`, the user's
  stream. Its flushes hold each chunk array's borrow while they write it,
  `for &chunk in chunks.borrow().iter() { write_chunk(destination, chunk)
  }`, and without the mark every chunk array's borrow in the crate is
  counted (ADR 0863), about 250 lines of `$borrowMut` in `try`/`finally`
  where upstream pushes to plain arrays. Only a destination whose `write`
  calls back into the same request at once and pushes to the array being
  flushed tells them apart: Rust would panic `already borrowed`, and the
  JS does what upstream's does, its `for` reaching the pushed chunk. So
  it's harmless: what runs is upstream's behavior, and Node's and the
  web's streams call back later, on `drain`, not in `write`. No compiler
  change: the mark is the port's. Held by
  `test/lowering-cell-borrows.test.ts`'s "a write binding marked as running
  nothing leaves a flushed array unchecked". Listed in the guide's
  differences (`semantics.md`, 38).
- **A guard that's only a call's receiver is held after the call's
  arguments run**, the second case's, which the user decided (2026-11-01,
  gap rd360, ADR 0863, amended): `target.borrow_mut().push(show(value));` is
  `target.push(show(value))`, which JS evaluates in that order, as React
  DOM's Fizz config's `stringToChunk(escapeTextForBrowser(value))` of a
  prop does. Rust takes the guard first. Only an argument whose code
  rust-js can't see, which calls back at once and borrows the receiver's
  cell, tells them apart: Rust panics `already borrowed`, and the JS does
  what upstream's does. A borrow rust-js sees in an argument still meets
  the guard. Held by `test/lowering-cell-borrows.test.ts`'s "a guard
  that's only a call's receiver is held while the call runs, not its
  arguments". Listed in the guide's differences (`semantics.md`, 39).
- **A guard read only for a binding's argument is held while it's read,
  not while the binding runs**, the second case's, which the user decided
  (2026-11-02, gap rd427, ADR 0863, amended): `prepare(loading,
  response.nonce.borrow().as_deref());` is `prepare(loading,
  response.nonce)`, as React's Flight client's
  `prepareDestinationForModule(response._moduleLoading, response._nonce,
  ..)`, and `js::set(target, "_debugInfo", chunk.debug_info.borrow()
  .clone())` its DEV `then`'s `wrapperPromise._debugInfo =
  this._debugInfo`. Rust holds the guard until the statement ends. Only
  a binding that calls back at once and borrows that cell mutably tells
  them apart: Rust panics `already borrowed`, and the JS passes. What it guards lent, `as_deref()`'s, or a
  copy or clone of it, given straight to the binding; anything else, and
  a later argument's code, is asked as before. Held by
  `test/lowering-cell-borrows.test.ts`'s "a guard read only for a
  binding's argument is held while it's read, not while the binding
  runs" and the corpus's `argument_guard_reentered`, against native Rust.
  Listed in the guide's differences (`semantics.md`, 44).
- **A global `static` tested again inside the `if` that tested it is
  decided there**, the first case's, which the user accepted (2026-10-28,
  gap pt6): React's `if (__DEV__) { .. if (__DEV__ && debugTask) { .. } }`
  is `if (debugTask != null)` inside, tidy's decided tests (ADR 0564): an
  immutable extern `static` reads the same each time, so `__DEV__ && x`
  has `x`'s truth there and the `if` takes the same branch; nothing tells
  them apart. Said in the guide's global values
  (`interop/global-scope.md`).
- **A class that extends nothing is a constructor function and its
  prototype**, which was the second case's, decided by the user
  (2026-11-01, gap rd331), is no case now: ADR 0761's form differs from an
  ES class only to JS that calls it without `new` or enumerates its
  prototype, and the ports whose upstream wrote an ES class, react-art's
  LinearGradient, RadialGradient and Pattern, mark their structs
  `#[rust_js::class(es)]` (ADR 0761, amended, gap rd682), as a port does
  where upstream writes one. The guide's difference 37 is gone (the
  coordinator's decision on the user's behalf, 2026-11-04).
- **A closure given as a descriptor's `get` is an arrow**, the second
  case's, which the user decided (2026-11-01, gap rd331f): react-art's Text
  defines `width` and the rest on its instance, `Object.defineProperty(this,
  key, { get: function () { return this._text ? this._text[key] :
  undefined; } })`, which the port's `PropertyDescriptor { get: Some(&||
  text._text.get().and_then(|text| js::get(text, *key))), .. }` writes
  `get: () => this._text?.[key]`. The arrow's `this` is the instance it
  was made in; the function's is what the property is read through, the
  instance too, but for a read through an object that inherits from it,
  `Object.create(text).width`, which no React caller makes. `_text` is a
  node or none, so `?.` gives what the ternary does. Listed in the guide's
  differences (`semantics.md`, 40).
- **A ref callback that sets a field returns `undefined`**, the second
  case's, which the user decided (2026-11-01, gap rd331): `ref={move |r|
  self._tag_ref.set(r)}` is `ref={(r) => { this._tagRef = r; }}`, a block,
  as `Cell::set` gives `()`, where react-art's `ref={ref => (this._tagRef
  = ref)}` returns the element, or `null` as it's detached. React reads a
  ref callback's result only where it's a function, React 19's cleanup;
  an element or `null` isn't one, and React 19's development build warns
  of neither, so React does the same. Only a caller that calls the
  callback itself and reads its result tells them apart. Listed in the
  guide's differences (`semantics.md`, 41).
- **An object's `None` field is no key where upstream writes `key:
  undefined`**, the second case's, which the user decided (2026-11-01,
  gap rd391, ADR 0280): React's Flight client's element and lazy node
  literals, `{$$typeof, type, key, props, _owner: undefined}` say, are
  `{ $$typeof, type, key, props }` of a struct whose field is `None`.
  A read of the key gives `undefined` either way, and JSON leaves it out
  either way; only `"key" in o`, `Object.keys(o)`, `hasOwnProperty` and a
  spread over another object's key tell them apart, which React's
  readers of these objects don't do. Listed in the guide's differences
  (`semantics.md`, 42).
- **A returned closure is an anonymous function where upstream names
  it**, the second case's, which the user decided (2026-11-02, gap rd335):
  react-art's createEventHandler is `return function handleEvent(event) {
  .. }`, whose port's `function::with_this(move |_, (event,)| { .. })`,
  returned, prints `return function (event) { .. }`. The function's
  `name` is `''` where upstream's is `'handleEvent'`. Only a stack trace,
  a DevTools listener list or a read of `.name` tells them apart, which
  React's code doesn't do; calling it does the same. Listed in the guide's
  differences (`semantics.md`, 43).
- **A generic function's instance may be two functions**, the second
  case's, which the user decided (2026-11-08, #987, ADR 0395, amended):
  `shown::<T>` taken as a value in generic code, given its caller's
  dictionaries, is the arrow made where it's taken, one for each time its
  maker runs, so `pick::<u8>() == pick::<u8>()` is `false`; and an
  instance its module knows whole is that module's `const`, one per
  module, so `shown::<u8>` taken in two modules is two functions (#974).
  Rust promises neither answer: rustc's
  `unpredictable_function_pointer_comparisons` lint warns of `==` of two
  function pointers, and std's `ptr::fn_addr_eq`, which it suggests, says
  the same function may have different addresses and different ones the
  same. Only a comparison of the pointers tells them apart; calling one
  does the same. Pinned by `test/lowering-fn-identity.test.ts`. Listed in
  the guide's differences (`semantics.md`, 47).
- **Two boxes of a value of no size made a `dyn Fn` are apart**, the
  second case's, which the user decided (2026-11-09, #1053, ADR 0959,
  amended): where the crate reads a `dyn Fn`'s address, every `Box`,
  `Rc` and `Arc` made one of a function another may hold too is a
  function of its own, as two allocations are apart to Rust. A `Box` of
  a function item, or of a closure that captures nothing, allocates
  nothing, and native Rust's two compare equal, `true`; JS's are `false`,
  as are two `make()`s of `Box::new(|x| x + 1)`, two arrows, which they
  were before. Generic code can't tell a value of no size from one that
  has a size, and Rust promises no address of one. Only a comparison of
  the addresses tells them apart; calling them does the same. Pinned by
  `test/lowering-fn-identity.test.ts`. Listed in the guide's differences
  (`semantics.md`, 48).
- **How often a hash map calls its key's own `hash` and `eq` is
  rust-js's**, the second case's, which the user decided (2026-11-07,
  difference 55, #913, ADR 1120). The user decided it of ADR 0168's
  map, which never called a key's `hash`; ADR 1120 calls it on each
  search, so what's left is how often: hashbrown hashes every key again as
  its table grows and compares keys whose 7 hash bits meet, rust-js hashes
  a key once a search and a clone's keys again. Matching it would mean
  copying hashbrown's growth and probing. Which keys a map holds is std's;
  a `hash` that prints or counts sees the difference only in a table that
  grows or is cloned. Pinned by corpus case
  `map_entry_hashes_its_key_once` (a table made `with_capacity`, each
  search's calls counted as std's). Listed in the guide's differences
  (`semantics.md`, 55).
- **A library's public re-export of std's item in no JS module exports
  nothing**, the second case's, which the user decided (2026-11-11, difference 50,
  #1315, ADR 0240, amended). Zerocopy's `pub use core::mem::*;`, or a
  named `pub use core::mem::forget;`, is in its module's JS as a type's
  or a trait's re-export is: not at all. A Rust consumer naming the item
  through the re-export calls std's own, as rust-js knows a std function
  wherever it's named. A JS caller of the library's module doesn't find
  `forget` there; it has none to call, as no JS module of rust-js's has
  it. Pinned by corpus case `std_module_reexported_by_a_glob` and crate
  pair `std_items_reexported_by_a_library`. Listed in the guide's
  differences (`semantics.md`, 50).
- **A `File` is closed once it's collected, not where Rust drops it**, the
  second case's, which the user decided (2026-11-11, difference 51, #1156, ADR 0425,
  amended). rustc checks a program for wasm32-unknown-unknown, whose `File`
  drops nothing, so its MIR has no drop of one; one `FinalizationRegistry`
  closes its descriptor once JS collects it, and a `ReadDir`'s directory
  too. A program holding more files than the OS allows at once, counting
  those Rust dropped and JS hasn't collected, sees it, and so does one that
  prints a `File`'s descriptor, `{:?}`'s `fd` (#1397), which isn't freed for
  the next file opened as Rust's is. Exact close at Rust's drop is #1391 (P2). Listed in
  the guide's differences (`semantics.md`, 51).
- **`{:?}` of a `File` shows where its path is now**, the second case's,
  which the user decided (2026-11-11, difference 54, #1397, ADR 0425,
  amended): std's `File { fd, path, read, write }`, its `path` the path it
  was opened by as it resolves where it's printed, `fs.realpathSync`'s, left
  out where nothing's there now, as std's leaves out one it can't say. A file
  renamed or removed since it was opened shows no `path`, where macOS's
  `F_GETPATH` shows a renamed one's new path and a removed one's last, and
  Linux's `/proc/self/fd` the new path or the old one with ` (deleted)`; one
  whose old path holds another file now shows that path. Its `fd` is Node's
  descriptor, as Rust's own differs by environment. Only a program that
  prints a file renamed or removed since it was opened sees it. Listed in
  the guide's differences (`semantics.md`, 54).
- **An error of the OS's of a code the runtime's table hasn't is
  `Uncategorized` with Node's words**, the second case's, which the user
  decided (2026-11-11, #1156, ADR 0462, amended): std's table of kinds is
  copied whole, with each code's `strerror` message, so only a code std
  doesn't map either, and that the table hasn't, `ENXIO` say, reads
  `no such device or address (os error 6)` where Rust's reads `No such
  device or address (os error 6)`. Listed in the guide's differences
  (`semantics.md`, 52).
- **A library's `dyn Error`, and the `io::Error` that holds one, drops what
  the library and its dependencies make one of**, the second case's, as
  ADR 0691's `dyn Any` does, which the user decided (2026-11-11, #1403,
  ADR 0462, amended): one a consumer makes of its own value with a
  destructor, and gives the library to drop, isn't dropped there. A
  consumer's destructor in a `dyn Error` a library drops is rare, and a
  value not dropped is a leak, which safe Rust may do too. Listed in the
  guide's differences (`semantics.md`, 53).
- **A writer that panics part-way through a `println!`'s line, in a
  program that catches no panic, loses what that line had written**, the
  second case's, which the user decided (2026-11-12, #1496, ADR 1130,
  amended): Rust's stdout keeps `x half` in its line buffer and writes it
  as the panicking program ends; the JS builds the line whole and loses
  it. Only the last line of a program that's ending on a panic differs,
  and only where its writer panics part-way, so it's rare, and harmless:
  the program has failed by then. Where a `catch_unwind` may catch the
  panic and the program prints on, the writer writes through stdout's
  line, as one that prints does, and the line is Rust's. Listed in the
  guide's differences (`semantics.md`, 57).

## Amendment: the stack's depth

A recursion deeper than the JS engine's stack, about 10,000 calls in
Node and browsers against Rust's 8 MB main thread, throws `RangeError`
where Rust's returns (#155). Results change and Rust's can't be kept:
no translation of a direct call gives JS a deeper stack, and how deep a
call goes is its input's, so no compile error can say where. It's the
fourth case's, as near as it gets: listed among the guide's differences
(`semantics.md`, 16), with what to write instead, an explicit stack.
Amended (#1193): what it does then is Rust's abort, exit 134, no drop run,
as where both overflow, not a panic `catch_unwind` takes
([0692](0692-a-thrown-error-is-a-panic-and-what-js-threw.md), amended).

## Amendment: the crate's iterator's own `fold` and `nth`

std routes `sum`, `count`, `for_each` and `last` through an iterator's
`fold`, and `skip` and `step_by` through its `nth`, so an impl that
writes its own runs it; rust-js steps the crate's iterator by its `next`
alone (#162). Results change only where the impl's own `fold` or `nth`
does something its `next` doesn't, a side effect or another answer, which
std's contract doesn't forbid but no iterator is written for: listed
among the guide's differences (`semantics.md`, 18), the issue open for
whether it's kept.

## Amendment: a string's `repeat` past the engine's length

A `str`'s `repeat(n)` is JS's own, unchecked
([0774](0774-room-past-isize-max-bytes-panics.md), amended), as the user
decided: a repeat past `isize::MAX` bytes, where std panics `capacity
overflow`, throws the engine's `RangeError`, in V8 before any string std
would refuse is made; in JavaScriptCore, whose strings are longer, a
string of text not all ASCII past a billion units may be made where std
panics. Results change only for a program that asks a string that long,
which runs out either way but in that corner: the second case, listed
among the guide's differences (`semantics.md`, 24).

## Amendment: a thread-local of std's or a binding's calls, made as its module loads

Rust makes a thread-local at its first use; rust-js makes one whose
initializer runs the crate's code there too (ADR 0037, amended). One whose
initializer calls only std's and bindings' functions is made as its module
loads, as JS writes it, React's `export const TocContext =
createContext([])`, which the user chose (2026-10-25) over making each
lazy, `TocContext.value` read through a holder. Results change only where
such a call does something a program sees before the thread-local's first
use, or a std call in it panics: the second case's, listed among the
guide's differences (`semantics.md`, 25).

## Amendment: room std's growth asks, and a collect of smaller items

Room for a length that's asked is checked as std checks it, `resize`'s,
a range's `collect()`'s and a hash table's (#190,
[0774](0774-room-past-isize-max-bytes-panics.md), amended). Room std's
own growth asks isn't: a `push`, `extend` or `insert` past the capacity
its doubling picks, which rust-js doesn't model; nor a `collect()` whose
length is a collection's of smaller items, `ids.iter().map(|x|
x.to_string())`, which the user chose (2026-10-25) over a `$roomFor`
before each such `collect()`. Results change only past `isize::MAX`
bytes, of hundreds of millions of items, or of items each a mebibyte
made of little: the second case, listed among the guide's differences
(`semantics.md`, 26), with a count collected through another adapter
(#246) until it's checked.


## Amendment: a function handed to a binding

A value with a destructor that JS holds is never dropped (ADR 0098, the
guide's twelfth difference); so with a function a binding is given,
which JS keeps, and which carries no drop
([0873](0873-a-function-handed-to-js-carries-no-drop.md)). Results change
only where JS gives that function back to Rust, through another binding
or an export, and Rust drops it: what it holds isn't dropped. A binding
that may give a function back itself, `widen`'s `this`, isn't handed one.
The second case, listed in the twelfth difference, and held by
`test/lowering-functions.test.ts`.

## Amendment: a once-callback that passes its call on

A closure given to a binding's `FnOnce` callback, `then`'s, `setTimeout`'s,
`queueMicrotask`'s or any binding's `impl FnOnce` or `Box<dyn FnOnce>`
parameter, that only passes its call on to a function of the crate's, or a
closure, is that function, as the user decided (2026-10-26): `listen(|_|
noop())` is `listen(noop)`. The function is one value, the closure a fresh
one each time it's made: only comparing the callback tells them apart, and
JS calls a once-callback once and keeps no other to compare it with. The
second case, listed among the guide's differences (`semantics.md`, 27),
with what it isn't (ADR 0605, amended):

- **What JS gives the function is what the closure's caller gives**, so
  the function is it only where it reads no more than the closure gives
  it: the crate's function, which takes the arguments Rust gives it alone,
  a library's that rust-js compiled from its Rust too (ADR 0605's second
  amendment), or a closure. Not a binding's JS function, a `dyn Fn` or a generic `impl
  Fn`, which may be a JS caller's of more parameters, nor a function of
  rest arguments, which reads them all.
- **What the callback gives is read**, `then`'s value: the closure gives
  its call's, or `undefined` of a call of `()`, which the crate's function
  gives too.
- **An `Fn` or `FnMut` parameter's, a listener's, keeps its arrow**, as
  does a closure stored: JS may keep a listener and compare it,
  `removeEventListener`'s, as this decision's two closures given to
  `addEventListener` and `removeEventListener` are two. (Widened since,
  the user's decision, 2026-10-30, rd196, rd212: a closure that ignores
  what it's given is the function it calls, a listener's too, in
  Consequences above.)

Held by `test/lowering-passed-on.test.ts` and the corpus's
`once_callback_passed_on.rs`, against native Rust, and
`scripts/mutations/lower/passed_on.ts`.

## Amendment: a future that runs as it's made (#965, the user's decision, 2026-11-08)

ADR 0029 made a future a promise, which runs as it's made, where Rust's
runs once it's polled, and listed it as no exception here: a result that
changes, a print before another's or a body that ran unawaited, with no
user's decision. It's decided now by this ADR's table: kept where nothing
can tell (case N, a future awaited before anything that may be seen runs,
or whose body can't be seen), refused where results change (case R,
none preserves Rust's order in a promise): held past what may be seen and
awaited after, dropped unawaited, or given where its `.await` can't be
followed. A future given to a binding is the JS boundary's, a running
promise by design (ADR 0029, amended).


## Amendment: an exception's name is its difference's number (2026-11-11)

An exception the second case allows is named by its number in the
guide's one list of differences, `semantics.md`'s "difference 51", and
its issue: the guide's list already numbers each one, so the number is
the one name for it. An `rd` id is the React port's review gap, rd114 or
rd731, never an exception's own: two exceptions had taken rd802 and
rd803, ids the port's gaps already held, and are now differences 50 and
51 (the coordinator's decision, 2026-11-11). In the Consequences an `rd`
id is only the gap that led to an exception, written `gap rdN` or `gaps
rdN, rdM`. `test/docs-differences.test.ts` holds both: each exception of
the second case names a difference the guide has, and no `rd` id there
is anything but a gap.
