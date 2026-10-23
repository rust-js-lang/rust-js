# rust-js design docs

rust-js compiles Rust to readable JavaScript. The whole design follows from one
sentence, borrowed from ReScript:

> Keep the language's own front end. Replace only the back end. Where the
> language and JS disagree, pick the JS behavior that keeps the output small
> and readable, and write that choice down.

To write an app in rust-js, read [the guide](guide/README.md): it says
what's current. This folder is where we write the choices down, and why. How a disagreement is
decided, by what the program can observe: [0262](decisions/0262-when-rust-and-js-disagree.md).

For what Rust does under rust-js today, read
[How Rust behaves in rust-js](guide/semantics.md), and for what it refuses,
[the known limitations](guide/limitations.md). The decisions below record why,
as each was made: a later one may lift an earlier one's "not yet" or "still
an error", and the page says what's current.

## The pipeline in one picture

```
 fib.rs
   │
   ▼
 rustc front end  (parse, expand macros, resolve names, type check)
   │
   ├──► THIR of every function ──copy──┐        (0004)
   ├──► MIR of every body ─────copy────┤        (0364)
   ▼                                   │
 rustc analysis (borrow check, lints)  │
   │                                   │
   ├── any error? ──► stop, write nothing  (0006)
   │                                   │
   ▼                                   ▼
 Compilation::Stop               mir.rs: MIR ──► JS AST, with spans  (0364)
 (no rustc codegen)                    │  THIR read beside it: names, spans, facts
                                       ▼
                                 mir/tidy.rs: written as a person writes it
                                       │
                                       ▼
                                 link.rs, prepare.rs: linked, then readable
                                       │
                                       ▼
                                 to_oxc.rs: JS AST ──► oxc AST           (0018)
                                       │
                                       ▼
                                 format.rs: laid out ──► one .js + .js.map per module  (0018, 0019)
```

## Code map

What each part of the code is, and where, is
[architecture.md's "Where the code is"](architecture.md#where-the-code-is):
the one map, beside the boundaries it keeps and where a change goes.

## Decisions

Each record says what we decided, why, what we rejected, and what it costs.

- [0042 Compiler boundaries and build-tool manifest](decisions/0042-compiler-boundaries-and-build-contract.md)

**Foundation**

- [0001 Reuse rustc's front end](decisions/0001-reuse-rustc-front-end.md)
- [0002 Generate JS from THIR, not MIR](decisions/0002-generate-from-thir.md) *(superseded by 0364)*
- [0003 Pin one nightly and link rustc's internals](decisions/0003-pin-nightly-toolchain.md) *(amended by 0109: the pin is a stable release)*
- [0004 Copy THIR before analysis, stop before codegen](decisions/0004-driver-hook.md)
- [0005 One file in, one ES module out](decisions/0005-input-and-output.md)
- [0006 Only programs rustc accepts become JS](decisions/0006-errors-and-unsupported-features.md)

**Code generation**

- [0007 A JS AST with a precedence-aware printer](decisions/0007-js-ast-and-printer.md) *(printer half superseded by 0018)*
- [0018 Print with oxc, through one adapter file, and emit source maps](decisions/0018-print-with-oxc.md)
- [0008 Two lowering modes: expressions and statements](decisions/0008-expression-and-statement-modes.md)
- [0009 Temporaries keep Rust's evaluation order](decisions/0009-temporaries-and-evaluation-order.md)
- [0010 Unique names per function, flat blocks](decisions/0010-naming-and-scopes.md)
- [0019 One JS file per Rust module](decisions/0019-one-js-file-per-module.md)

**Semantics**

- [0011 Integers are JS numbers, wrapped like release Rust](decisions/0011-numbers.md)
- [0012 Panics throw, via runtime helpers emitted on demand](decisions/0012-panics-and-runtime-helpers.md) *(helpers' place replaced by 0103; amended by 0660: `catch_unwind` catches one)*
- [0013 Fieldless enum variants are strings](decisions/0013-fieldless-enums.md)
- [0033 Enums with fields are ReScript's tagged objects](decisions/0033-enums-with-fields.md)
- [0035 JS that throws is a `Result`; `?` returns early](decisions/0035-results-and-throwing-js.md) *(amended by 0637: a `match` of its `Result` is `try` and `catch`; 0643: its `unwrap()` is the call)*
- [0020 Structs are objects, tuples are arrays](decisions/0020-structs-and-tuples.md) *(amended by 0674: a literal's fields are listed as written)*
- [0049 Trait dictionaries, generics, and read-only trait objects](decisions/0049-traits-and-generics.md)
- [0047 Methods are an object of functions named after their type](decisions/0047-methods.md)
- [0014 `match` becomes an `if`/`else if` chain](decisions/0014-match-lowering.md) *(amended by 0629: a `match` of constants is a `switch`)*
- [0048 Let chains: each part runs only once the ones before it held](decisions/0048-let-chains.md)
- [0051 `Option<T>` in generic code: boxed only when it looks like `None`](decisions/0051-generic-options.md)
- [0052 The crate's own `Default`, `From` and `Clone`, and the trait ABI kept](decisions/0052-std-trait-impls.md)
- [0053 `==`: JS's `===` or `$eq`, until a hand-written `eq` is in it](decisions/0053-partial-eq.md)
- [0054 `Display`: a `fmt` returns the string it writes](decisions/0054-display.md)
- [0055 The crate's own `Iterator` is a JS iterator](decisions/0055-iterator.md)
- [0056 Indexing: `$index(v, i)` to read, `v[$at(v, i)] = x` to write](decisions/0056-indexing.md)
- [0057 `PartialOrd` and `Ord`: an `Ordering`, and the parts in turn](decisions/0057-ordering.md) *(amended by 0687: `max()` and the rest step their iterator as each comes)*
- [0058 Format options, where Rust applies them](decisions/0058-format-options.md)
- [0059 `HashMap` is a JS `Map`, `HashSet` a `Set`, keyed by value](decisions/0059-hashmap.md)
- [0060 `{:?}` by the type, and a derived `Debug` is a function](decisions/0060-debug.md)
- [0061 `impl Iterator` is the type it hides; a generic iterator is any JS iterable](decisions/0061-generic-iterators.md)
- [0062 Combinators and adapters: the closure's body in place](decisions/0062-combinators.md)
- [0063 `char`'s questions are Unicode regular expressions; `parse` is a `Result` of Rust's message](decisions/0063-text.md)
- [0064 Numbers' methods are `Math`'s, where JS agrees; operators call their impl](decisions/0064-numbers.md)
- [0086 An `i64` or a `u64` is a BigInt, wrapped as release Rust wraps it](decisions/0086-64-bit-integers.md)
- [0087 `println!` is `console.log`; `print!` writes as it is where JS can](decisions/0087-printing.md)
- [0090 rustc checks programs for `wasm32-unknown-unknown`, whose `usize` is rust-js's](decisions/0090-wasm32-front-end.md)
- [0065 The JS is formatted as oxfmt formats it, and the source map follows](decisions/0065-format-with-oxfmt.md)
- [0066 Text with values in it is a template literal](decisions/0066-template-literals.md) *(amended by 0641: a `concat!` of literals is its pieces joined by `+`)*
- [0067 Range patterns, `@`, `let ... else`, and a `&mut` into a map](decisions/0067-patterns.md)
- [0069 Preserve effects before simplifying; lower once and link afterwards](decisions/0069-lowering-effects-and-linking.md)
- [0070 A std function taken as a value is an arrow](decisions/0070-function-values.md)
- [0071 An iterator stepped through is a `$iter`, which knows where it is](decisions/0071-stepping-iterators.md)
- [0074 A `&mut` to a string or a number is a box the caller copies back](decisions/0074-mut-boxes.md)
- [0076 A struct's `..base` after its fields, and std's own steps for a few more methods](decisions/0076-std-odds.md)
- [0068 `VecDeque` and `BinaryHeap` are arrays; a heap moves its items as Rust's does](decisions/0068-queues.md)
- [0015 Loops: put `while` back, label only when needed](decisions/0015-loops.md)

**Web programs**

- [0021 JS interop: `extern` blocks name what JS has](decisions/0021-js-interop.md)
- [0022 Closures are arrow functions](decisions/0022-closures.md)
- [0023 Strings, references and shared state](decisions/0023-strings-references-shared-state.md)
- [0024 The `webapi` crate: DOM bindings generated from WebIDL](decisions/0024-web-crate.md)
- [0025 `Vec`, `for` loops, `RefCell` and `&mut` to objects](decisions/0025-vec-loops-refcell-mut.md)
- [0034 String methods are JS's; a `char` is a one-character string; `format!` is `+`](decisions/0034-strings-and-chars.md)
- [0036 An iterator is a JS array; `Ordering` is -1, 0 or 1](decisions/0036-iterators-and-sorting.md)
- [0037 `thread_local!` is a variable of its module](decisions/0037-thread-locals.md)
- [0028 Imports from JS modules: `#[link_name = "module#path"]`](decisions/0028-js-module-imports.md)
- [0029 `async`/`.await` are JS's `async`/`await`; a future is a promise](decisions/0029-async-await.md)
- [0030 `Option`: `Some(x)` is `x`, `None` is `undefined`](decisions/0030-option.md)
- [0031 A `const` is the value rustc computed, under its own name](decisions/0031-consts.md)
- [0038 Variables have JS's names and shapes: `const [count, setCount] = ..`](decisions/0038-js-names-and-destructuring.md)
- [0039 Generic bindings: `#[rust_js::link_name]` on an ordinary function](decisions/0039-generic-bindings.md)
- [0046 `#![rust_js::camel_case]`: a crate's own names, the JS way](decisions/0046-camel-case-crates.md) *(superseded by 0435)*

**React**

- [0040 JSX: bindings whose `link_name` is a tag, printed as JSX in a `.jsx` file](decisions/0040-jsx.md)
- [0041 React: the `react` crate, and Vite with Fast Refresh](decisions/0041-react.md)
- [0043 React's whole API, gated by the release that added it](decisions/0043-react-versions.md)
- [0072 JSX syntax in the native and browser compilers](decisions/0072-jsx-syntax.md)
- [0075 JSX is the public syntax for React elements](decisions/0075-jsx-only-elements.md)

**Talking to a server**

- [0077 `#[derive(Serialize)]` is a function that writes serde_json's text](decisions/0077-serde-json.md)
- [0078 `serde_json::from_str` is serde_json's reader, ported; `#[derive(Deserialize)]` is a table it reads by](decisions/0078-serde-json-reading.md)
- [0079 A tagged or untagged enum is read as serde reads one: through the value, read first and kept](decisions/0079-serde-tagged-enums.md)
- [0080 A generic type's codec takes its type parameters' codecs; `Result`, `from`, `try_from` and `into` are serde's](decisions/0080-serde-generics-and-conversions.md)
- [0081 A `T: Serialize` bound's evidence is `T`'s writer, and `T: DeserializeOwned`'s its reader](decisions/0081-serde-bounds.md)
- [0082 `#[serde(flatten)]` is serde's flat map: a field's entries among its struct's](decisions/0082-serde-flatten.md)
- [0083 serde_json's `Value` is an enum like any other; its `Number` is `{ kind, value }` and its `Map` a `Map`](decisions/0083-serde-json-value.md)

**Scope and process**

- [0016 Only top-level functions, for now](decisions/0016-crate-shape.md) *(modules: superseded by 0019)*
- [0017 Test against native Rust, not against expectations](decisions/0017-differential-testing.md)
- [0088 The corpus: Rust programs that say what they expect, run natively and as JS](decisions/0088-corpus.md)
- [0089 rustc's own `run-pass` tests run as JS, with a list of what fails that only shrinks](decisions/0089-rustc-tests.md)
- [0091 What rust-js writes depends only on what it's given](decisions/0091-build-history.md)
- [0092 Generated programs, each from a seed, and reduced when they fail](decisions/0092-generated-programs.md)
- [0093 Known bugs put back into the compiler, which the tests must catch](decisions/0093-mutations.md)
- [0094 A distribution is qualified by the suite, run through what it installs](decisions/0094-qualification.md)
- [0095 The JS rust-js makes runs on Node; Bun runs the tests](decisions/0095-node-runtime.md)
- [0096 A `static` is its module's value, and a `static mut` its `{ value }`](decisions/0096-statics.md)
- [0097 A user `Deref` or `IndexMut` is its method, and an auto trait's impl is nothing](decisions/0097-deref-and-marker-impls.md)
- [0098 A destructor runs where rustc runs it, in a `finally`](decisions/0098-destructors.md) *(amended by 0660: a panic's drops run before `catch_unwind` returns)*
- [0099 A `&mut` held in a variable names its place; one kept elsewhere is a handle](decisions/0099-mut-references.md)
- [0100 Each crate is compiled once, to JS of its own, with a manifest of what crosses](decisions/0100-separate-crates.md)
- [0101 Cargo builds a workspace with rust-js as its workspace wrapper](decisions/0101-cargo-workspace-wrapper.md)
- [0102 Two binding crates, `js` and `webapi`, named as ReScript's](decisions/0102-js-and-webapi.md)
- [0103 The runtime is a package, `@rust-js/runtime`, as ReScript's is](decisions/0103-runtime-package.md)
- [0104 The tests run side by side, each native program built once](decisions/0104-parallel-tests.md)
- [0105 `bun create @rust-js` makes a Vite and React app in Rust](decisions/0105-create.md)
- [0106 A trait's type parameters, associated types, generic methods and constants](decisions/0106-generic-traits.md)
- [0107 A const parameter is a value its caller gives](decisions/0107-const-generics.md)
- [0108 Operators and `Into` in generic code are dictionaries](decisions/0108-generic-operators-and-into.md)
- [0109 Pin a stable release: rust-js checks code as the Rust most people run does](decisions/0109-stable-release.md)
- [0110 rust-js's syntax is stable Rust's](decisions/0110-stable-syntax.md)
- [0111 A JS type is a struct of a `JsObject`](decisions/0111-js-types-as-structs.md)
- [0112 rust-js compiles the binding crates](decisions/0112-rust-js-compiles-the-bindings.md)
- [0113 A plain rustc compiles a program too](decisions/0113-plain-rustc.md)
- [0114 An app's Rust is a Cargo package, for editors](decisions/0114-app-cargo-toml.md)
- [0115 The binding crates are packaged for crates.io](decisions/0115-binding-crates-on-crates-io.md)
- [0116 How bindings are versioned](decisions/0116-binding-versions.md)
- [0117 A crate's settings for the JS rust-js writes: its formatter's options, and hooks](decisions/0117-output-hooks.md)
- [0118 Bindings on npm only, as ReScript's are](decisions/0118-bindings-on-npm-only.md)
- [0119 Bindings the community maintains, in one repository](decisions/0119-community-bindings.md)
- [0120 The first npm release: 0.0.1, for macOS on Apple silicon](decisions/0120-first-npm-release.md)
- [0121 A map keyed by a struct is a `Map` keyed by its value](decisions/0121-value-keys.md)
- [0122 An `f32` is a JS number, each result rounded with `Math.fround`](decisions/0122-f32.md)
- [0123 A slice's pattern tests its length and its items](decisions/0123-slice-patterns.md)
- [0124 A `|` pattern binds each name where its alternative has it](decisions/0124-or-pattern-bindings.md)
- [0125 A constructor, or a closure as a `fn`, is a JS function](decisions/0125-function-values.md)
- [0126 A byte string is its bytes, and `as_bytes()` a string's UTF-8 bytes](decisions/0126-byte-strings.md)
- [0127 An inline `const` is its value, and an `if let` guard binds for its arm](decisions/0127-const-blocks-and-let-guards.md)
- [0128 std's iterator sources: `once` is an array, and `repeat` a JS iterator](decisions/0128-iterator-sources.md)
- [0129 A range is a value: `{ start, end }`, iterated as its items](decisions/0129-range-values.md)
- [0130 An `if let` is a value anywhere, and trait and std functions are values](decisions/0130-if-let-values-and-fn-items.md)
- [0131 A temporary taken apart owns what its pattern leaves](decisions/0131-temporaries-taken-apart.md)
- [0132 A `Box` from its value, `Default` of a `&str`, `type_name`, and standard streams](decisions/0132-std-odds.md)
- [0133 Impls whose names would be the same are named by their arguments](decisions/0133-impl-names-apart.md)
- [0134 A `let` of a pattern without a value declares each of its variables](decisions/0134-let-patterns-without-values.md)
- [0135 A trait's const parameter is its impl's, and a trait method's is given](decisions/0135-const-generic-traits.md)
- [0136 Common std methods: `take`, `cmp::max`, ASCII case, and `Debug`'s builders](decisions/0136-common-std-methods.md)
- [0137 `{:#?}` is pretty Debug, and a writer is told whether](decisions/0137-pretty-debug.md) *(its `alternate` superseded by 0143)*
- [0138 A string's length, slices and offsets count its UTF-8 bytes](decisions/0138-string-byte-counts.md) *(amended by 0642: a place found in a string is JS's, `indexOf` and `slice`)*
- [0139 A chain whose stages do what can be seen runs in Rust's order](decisions/0139-lazy-chains.md)
- [0140 An iterator trait object is a JS iterator](decisions/0140-iterator-trait-objects.md)
- [0141 A `dyn Display` or `dyn Error` is a value and its dictionary](decisions/0141-std-trait-objects.md)
- [0142 A channel on one thread is a queue its ends share](decisions/0142-channels.md)
- [0143 A `Formatter`'s options are an object its writers are given](decisions/0143-formatter-options.md)
- [0144 A lock on one thread is a `RefCell`, and an `Arc` is an `Rc`](decisions/0144-locks.md)
- [0145 A generic function is given the size and name of a type parameter it asks for](decisions/0145-type-facts.md)
- [0146 A generic associated type of lifetimes is an associated type](decisions/0146-generic-associated-types.md)
- [0147 Replacing a value whole through a `&mut`: an object in place, an enum by its place](decisions/0147-replacing-through-mut.md)
- [0148 `write!` into a `String` adds to it, and a `fmt::Result` is always `Ok`](decisions/0148-write-to-string.md)
- [0149 A `String` changed in place is given a new string, by its byte offsets](decisions/0149-string-editing.md)
- [0150 Splitting and searching by a pattern, as Rust searches](decisions/0150-string-patterns.md)
- [0151 Any function taken as a value is the arrow that calls it](decisions/0151-called-function-values.md)
- [0152 A `&mut` std hands out to a number or a string is a handle on it](decisions/0152-std-item-handles.md)
- [0153 Collection and cell methods, and `to_vec()` cloning what it copies](decisions/0153-collection-and-cell-methods.md)
- [0154 Number methods, and parsing that stops where Rust's stops](decisions/0154-number-methods.md)
- [0155 An integer's checked, wrapping, overflowing and saturating families](decisions/0155-integer-families.md)
- [0156 An integer's bits rotated, its bytes, and a float's bits](decisions/0156-integer-bits-and-bytes.md)
- [0157 A closure, a function or a set of `char`s as a pattern, and a string's pieces by bytes](decisions/0157-char-predicates.md)
- [0158 A labeled block is JS's labeled block](decisions/0158-labeled-blocks.md)
- [0159 A type's own `FromStr` is what `parse` calls](decisions/0159-user-from-str.md)
- [0160 A collection of the crate's: its own `IntoIterator`, `FromIterator`, `Extend`, `Sum` and `Product`](decisions/0160-user-collections.md)
- [0161 `T: FromStr` in generic code: a dictionary, std's or the crate's](decisions/0161-generic-from-str.md)
- [0162 `AsRef` and `AsMut` in generic code: a dictionary, the value itself for std's](decisions/0162-generic-as-ref.md)
- [0163 A trait's generic method is given drops for its own type parameters, as its trait declares them](decisions/0163-trait-method-drops.md)
- [0164 An iterator of the crate's from both ends, and of a known length](decisions/0164-double-ended-iterators.md)
- [0165 `{:x}`, `{:e}` and `{:p}` of the crate's types call its own impls](decisions/0165-other-fmt-traits.md)
- [0166 A writer of the crate's is given its text a `write!` at a time](decisions/0166-user-fmt-write.md)
- [0167 `Borrow`: a dictionary, the value itself for std's](decisions/0167-borrow.md)
- [0168 A type's own `Hash` is allowed, and never lowered](decisions/0168-user-hash.md)
- [0169 A type's own `AsMut` and `BorrowMut`, called on the type](decisions/0169-user-as-mut.md)
- [0170 `size_hint()` where its answer is known](decisions/0170-size-hint.md)
- [0171 An `i128` or a `u128` is a BigInt, wrapped to 128 bits](decisions/0171-128-bit-integers.md)
- [0172 Bytes to text, as Rust validates UTF-8](decisions/0172-utf8-decoding.md)
- [0173 A path is its text](decisions/0173-paths-as-text.md)
- [0174 `{:x}` and `{:p}` of a generic `T`: its dictionary's](decisions/0174-generic-fmt-traits.md)
- [0175 `std::num::Wrapping` is a number in a `[x]`, its arithmetic wrapped](decisions/0175-wrapping.md)
- [0176 A generic impl's constant of its parameters is its dictionary's getter](decisions/0176-generic-impl-consts.md)
- [0177 A `NonZero` integer is its number](decisions/0177-nonzero.md)
- [0178 An impl's dictionary has its associated types' drops](decisions/0178-associated-type-drops.md)
- [0179 `filter` and `map_or` drop what they don't keep](decisions/0179-option-filter-map-or-drops.md)
- [0180 Generic code that writes to any writer is given a `fmt::Write` dictionary](decisions/0180-generic-writers.md)
- [0181 A clone of std's iterator over an array is what's left of the array](decisions/0181-iterator-clones.md)
- [0182 Boundary matrices: each operation on the values where implementations go wrong](decisions/0182-boundary-matrices.md)
- [0183 Strings and `char`s order by code point, and `trim` removes Unicode's White_Space](decisions/0183-code-point-order-and-trim.md)
- [0184 `ok_or_else` keeps its value, a branch's operand is dropped in its branch, and `by_ref()` of the crate's iterator](decisions/0184-bitflags-gaps.md)
- [0185 A library's trait's defaults are its functions over `Self`](decisions/0185-library-trait-defaults.md)
- [0186 What another crate's derive writes is the crate's own code](decisions/0186-other-crates-derives.md)
- [0187 A writer's `Err(fmt::Error)` is thrown, with what it wrote, and each consumer takes it as std's does](decisions/0187-fmt-error.md)
- [0188 A `Duration` is its nanoseconds, a BigInt](decisions/0188-duration.md)
- [0189 `panic!("{}", x)`, a slice into an array, and `into_iter()` of the crate's iterator](decisions/0189-panic-display-slice-arrays.md)
- [0190 Generic code may take no destructor of a type parameter, and its callers are checked](decisions/0190-generic-no-destructor.md)
- [0191 An or-pattern's moves, an arm's temporaries, and `Box::from` of a `&str`](decisions/0191-or-patterns-arm-temporaries-boxed-str.md)
- [0192 A Next.js app's routes and components in Rust, beside its JS](decisions/0192-next.md)
- [0193 `Some` of a unit variant is tested as the variant](decisions/0193-some-of-a-unit-variant.md)
- [0194 An attribute read before a child is read once](decisions/0194-jsx-attributes-read-once.md)
- [0195 The props a component's struct doesn't name are `...rest`](decisions/0195-rest-props.md)
- [0196 A crate may have a `.d.ts` beside each module's JS](decisions/0196-typescript-declarations.md)
- [0197 What's moved before anything can leave isn't its scope's to drop](decisions/0197-moved-before-leaving.md)
- [0198 An optional handler passed on to an element is that handler](decisions/0198-optional-handlers.md)
- [0199 A component takes no drop of its type parameters](decisions/0199-components-take-no-drops.md)
- [0200 A binding's props borrow their text, and take `aria-label` and the rest](decisions/0200-binding-props-borrow.md)
- [0201 A component takes no dictionary, and an update makes no default it replaces](decisions/0201-components-take-no-dictionaries.md)
- [0202 An import is named in each file that imports it](decisions/0202-imports-named-around-importers.md)
- [0203 A component's props are as written](decisions/0203-component-props-as-written.md) *(amended by 0674: the order is the object's)*
- [0204 A flattened field's struct is its parent's props](decisions/0204-flattened-props.md) *(amended by 0674: found by its name)*
- [0205 Flattened structs chain, and the props' own name is theirs](decisions/0205-flattened-chains.md)
- [0206 TypeScript, read and written by TypeScript's own parser and printer](decisions/0206-typescript-module.md)
- [0207 A crate's `.d.ts` is printed by TypeScript](decisions/0207-declarations-printed-by-typescript.md)
- [0208 Each element's attributes, as @types/react types them](decisions/0208-react-attributes.md)
- [0209 A two-arm `match` that's a value is a conditional](decisions/0209-match-conditional.md)
- [0210 A module's declarations import another's types](decisions/0210-declarations-import-module-types.md)
- [0211 `Option::as_deref` of a `String` or a `Vec` is the option](decisions/0211-option-as-deref.md)
- [0212 A props field's default is where JS takes them apart](decisions/0212-props-defaults.md)
- [0213 A component's props are given as one flat list](decisions/0213-props-as-written.md)
- [0214 An untagged enum is its payload: TS's `string | Blob`](decisions/0214-untagged-enums.md) *(amended by 0668: an `otherwise` of one `typeof` kind is tested as it; by 0676: a variant without fields may be `undefined` or `null`)*
- [0215 The webapi crate's unions are untagged enums](decisions/0215-webapi-unions.md)
- [0216 A slice's `concat` of parts written out is an array of them, spread](decisions/0216-concat-spread.md)
- [0217 `enumerate()` then a callback's method is the method, given each index](decisions/0217-enumerate-index.md)
- [0218 What JSX holds stays where it's written](decisions/0218-jsx-holds-in-place.md)
- [0219 A sequence a webapi function takes is a slice](decisions/0219-webapi-sequences.md)
- [0220 A JSX tag can be a value, named by a capitalized local](decisions/0220-jsx-tag-values.md)
- [0221 A binding's last slice can be its rest arguments](decisions/0221-variadic-bindings.md) *(amended by 0640: a TypeScript rest parameter is a tuple)*
- [0222 An editor checks the app through rust-js](decisions/0222-editor-check.md)
- [0223 The webapi crate knows each event's type and each tag's element](decisions/0223-webapi-event-and-tag-maps.md)
- [0224 A tag's element reaches its handlers and its `ref`](decisions/0224-typed-intrinsic-elements.md)
- [0225 A JS value of unknown shape is a `js::Unknown`](decisions/0225-unknown-values.md) *(amended by 0668: `Kind::Symbol`, and a `match` of `Kind` a `switch (typeof value)`; by 0673: `js::unknown` of a function; by 0676: `classify` takes an `Option<&Unknown>`, and `Kind` has `Undefined` and `Null`; by 0677: `Kind::Object` holds a `&JsObject`)*
- [0226 A component looks inside its children as React's `Children` does](decisions/0226-react-children.md)
- [0227 The react crate's types have @types/react's names](decisions/0227-react-type-names.md)
- [0228 A tag takes the attributes @types/react gives it](decisions/0228-per-tag-attributes.md)
- [0229 A union parameter takes each member as it is](decisions/0229-union-parameters.md)
- [0230 A bound of a trait with nothing in it passes no dictionary](decisions/0230-marker-bounds.md)
- [0231 A `matches!` of a kind's literal is the literal's test, in place](decisions/0231-matches-of-a-literal.md)
- [0232 A `let`-`else` of a `filter` tests the filter and binds what it kept](decisions/0232-let-else-of-a-filter.md)
- [0233 A `match` giving a table's field named as each variant reads the table by it](decisions/0233-match-reads-a-table.md)
- [0234 A component of an element's props is an `ElementType`, rendered as a tag](decisions/0234-element-type.md)
- [0235 A child shown only if a test holds is `test && child`, where the test can't render](decisions/0235-children-shown-if.md)
- [0236 What JSX makes is `JSX::Element`, TypeScript's `JSX.Element`](decisions/0236-jsx-element.md)
- [0237 An `if` whose branch leaves has no `else`](decisions/0237-no-else-after-leaving.md) *(amended by 0683: an `else if` of an arm that goes on stays one)*
- [0238 A value kept for good is the value; children are kept as JS values](decisions/0238-kept-values-and-children.md)
- [0239 A local of a component's function type is that component as a tag](decisions/0239-local-components.md)
- [0240 A `pub use` of another module's function is re-exported from it](decisions/0240-reexports.md)
- [0241 Text right before text is in braces](decisions/0241-adjacent-text.md)
- [0242 A `dyn ReactNode` is the node itself](decisions/0242-dyn-react-node.md)
- [0243 A RegExp of a pattern as it's written is a literal](decisions/0243-regex-literals.md)
- [0244 A value taken apart through a shared reference is destructured](decisions/0244-destructuring-through-references.md)
- [0245 next/legacy/image, and a module's imports named by its own](decisions/0245-legacy-image-and-module-import-names.md)
- [0246 A type parameter of a function needs no copy of its own](decisions/0246-function-parameters-need-no-copy.md)
- [0247 JS's string methods are the builtins crate's `string`](decisions/0247-js-string-methods.md) *(amended by 0642: for JS's own indexes, std's methods for a place found)*
- [0248 A `thread_local!` is a module's default export too](decisions/0248-thread-local-default-export.md)
- [0249 An element's key may be none](decisions/0249-optional-keys.md)
- [0250 Props updated from a reference are a spread](decisions/0250-reference-base-spread.md) *(amended by 0674: the fields after it as written)*
- [0251 A module's default export is imported as its default](decisions/0251-default-imports.md)
- [0252 A one-armed match's value is its body's](decisions/0252-one-arm-match-value.md)
- [0253 next/link takes a ref and passHref](decisions/0253-next-link-ref.md)
- [0254 A key or ref that does nothing is captured in no order](decisions/0254-pure-key-ref.md)
- [0255 next/router's events](decisions/0255-next-router-events.md)
- [0256 A namespace import is named as the module of its bindings](decisions/0256-namespace-import-names.md)
- [0257 A closure that only spawns an async block is an async arrow](decisions/0257-spawning-closure-async-arrow.md)
- [0258 A promise's then, a module of a component, next/head and the Router](decisions/0258-search-bindings.md)
- [0259 A module's constant element, and an image's title](decisions/0259-constant-elements.md)
- [0260 A binding's callback parameter, and IntersectionObserver](decisions/0260-webapi-callbacks.md) *(amended by 0653: a function JS gives is a `dyn Fn`)*
- [0261 Defaults that do nothing don't move what's made beside them](decisions/0261-defaults-made-in-place.md)
- [0262 When Rust and JS disagree, what the program can observe decides](decisions/0262-when-rust-and-js-disagree.md)
- [0263 next/router's withRouter, and a flattened struct taken apart](decisions/0263-with-router.md)
- [0264 A variant's own name is the variant](decisions/0264-enum-own-names.md)
- [0265 A component a thread_local! holds takes props of any lifetime](decisions/0265-held-component-lifetimes.md)
- [0266 Text kept where it isn't empty, or another, is ||](decisions/0266-text-or.md) *(amended by 0635: preparation writes the `||`, of every fallback)*
- [0267 js::on_load! is what a module runs when it's loaded](decisions/0267-on-load.md)
- [0268 An element's type, key and props may be JS values of any shape](decisions/0268-unknown-elements.md)
- [0269 What every global scope has is called bare](decisions/0269-global-scope.md)
- [0270 A thread-local only read and set is its module's variable](decisions/0270-module-variables.md)
- [0271 A JS value's truthiness, and a value vouched to be a type](decisions/0271-truthy-and-cast.md)
- [0272 Node's modules are a crate of their own, typed as @types/node](decisions/0272-node.md)
- [0273 A `#[path]` module's JS is beside its file](decisions/0273-path-modules.md)
- [0274 Next.js's getStaticProps and getStaticPaths, typed as next types them](decisions/0274-next-data-fetching.md)
- [0275 A nullable field's `None` is `null`](decisions/0275-nullable-writes-null.md) *(amended: a nullable parameter's `Some(..)` is its value)*
- [0276 A Pages Router route gets no declarations beside it](decisions/0276-no-declarations-in-routes.md)
- [0277 A variable shadowed by its own value is the variable](decisions/0277-shadow-aliases.md)
- [0278 A variable nothing sets again is a `const`](decisions/0278-unset-lets-are-consts.md)
- [0279 A callback that passes on what it's given is the function](decisions/0279-callbacks-passed-on.md)
- [0280 A field that's `undefined` is no key](decisions/0280-undefined-fields-left-out.md)
- [0281 The webapi crate covers TypeScript's DOM, and a test holds it](decisions/0281-webapi-covers-typescripts-dom.md)
- [0282 One method API for event listeners](decisions/0282-event-listener-methods.md)
- [0283 The js crate covers TypeScript's ES library, and a test holds it](decisions/0283-js-crate-covers-typescripts-es-library.md) *(amended by 0666: descriptors and handlers are dictionaries, `Object` measured; by 0677: a weak collection is typed as TypeScript's)*
- [0284 An enum tagged by a property of its own is a discriminated union](decisions/0284-discriminated-unions.md)
- [0285 `std::ptr::eq` of JS objects is whether they're one](decisions/0285-ptr-eq.md)
- [0286 A component that sometimes renders nothing returns an `Option`](decisions/0286-components-that-render-nothing.md)
- [0287 A cell only its function reads and sets is a `let`](decisions/0287-cells-as-variables.md)
- [0288 A `Cell` in a field is the property it holds](decisions/0288-cell-fields.md)
- [0289 A `Copy` bound takes a copy only where a copy isn't the value](decisions/0289-copy-bounds.md)
- [0290 A pattern names a variable set again while nothing sets it](decisions/0290-steady-bindings.md)
- [0291 A `let` taking a variable apart is JS's destructuring](decisions/0291-let-destructures-places.md)
- [0292 An index a condition shows in bounds is read as it is](decisions/0292-known-bounds.md)
- [0293 A `Cell` a `let` takes apart and reads at once is its value](decisions/0293-cells-read-at-once.md)
- [0294 `unwrap_unchecked()` is the value, as TypeScript's `x!` is](decisions/0294-unwrap-unchecked.md)
- [0295 A module's imports are one block, packages first](decisions/0295-import-order.md)
- [0296 A function a block makes and gives is a named function expression](decisions/0296-named-function-expressions.md)
- [0297 A closure a `let` names for a hook is the hook's named function](decisions/0297-named-hook-functions.md)
- [0298 An Option of a value never falsy is tested by its truth](decisions/0298-truthy-options.md)
- [0299 A return of `undefined` is a bare `return;`](decisions/0299-bare-return.md)
- [0300 A function takes the drops its body uses](decisions/0300-drops-by-need.md)
- [0301 A read that can't throw comes before a move](decisions/0301-reads-that-cannot-throw.md)
- [0302 An `f64` known to be a whole number in range is cast as it is](decisions/0302-whole-number-casts.md)
- [0303 Top-level statements that span lines are set apart](decisions/0303-blank-lines.md)
- [0304 `js::import!` of an item loads its module when it's asked for](decisions/0304-dynamic-import.md)
- [0305 A binding marked `require` is read through `require(module)`](decisions/0305-require.md)
- [0306 A module's items are where its Rust has them](decisions/0306-item-order.md)
- [0307 A trait's binding method is read of the value itself](decisions/0307-binding-trait-methods.md)
- [0308 A function written in a body is that body's](decisions/0308-local-functions.md)
- [0309 A `match` that only gives a value is one expression](decisions/0309-value-matches.md)
- [0310 Any value is read as JS reads it](decisions/0310-any-value-as-js-reads-it.md) *(amended by 0639: `concat_text` of any value may throw)*
- [0311 `.flatten()` of a read of items that may be `None` is the read](decisions/0311-flattened-reads.md)
- [0312 `js::shown` is any value as a template shows it](decisions/0312-shown.md)
- [0313 Loops, spreads and calls as a person writes them](decisions/0313-console-forms.md) *(amended by 0640: `console` takes a tuple)*
- [0314 Each std data structure works or is refused, and coverage only grows](decisions/0314-std-data-structures.md)
- [0315 A `Vec`'s capacity does nothing, and its edits are std's](decisions/0315-vec-capacity-and-edits.md)
- [0316 A `LinkedList` is an array, as a `VecDeque` is](decisions/0316-linked-list.md)
- [0317 A `OnceCell` is a `{ value }` of an `Option`, set once](decisions/0317-once-cells.md)
- [0318 A `LazyCell` makes its value when it's first used, as Rust's does](decisions/0318-lazy-cells.md)
- [0319 A `Cow` is its enum, made owned by a clone of what it borrowed](decisions/0319-cows.md)
- [0320 An `Rc` whose counts are read is counted, as Rust counts it](decisions/0320-counted-rcs.md)
- [0321 A map drops its values as Rust's does](decisions/0321-map-drops.md)
- [0322 Text's and slices' methods by their other names, and a join of lists](decisions/0322-text-and-slice-names.md)
- [0323 A `String`'s byte-range edits, and `str`'s searches from the end](decisions/0323-text-byte-ranges.md)
- [0324 A slice's fills, copies, order checks, chunks and splits](decisions/0324-slice-methods.md)
- [0325 Maps' and sets' own methods, a B-tree's in its keys' order](decisions/0325-map-and-set-methods.md)
- [0326 `Option`'s and `Result`'s other methods, and an `Option`'s place filled](decisions/0326-option-and-result-methods.md)
- [0327 Cells', locks', a `VecDeque`'s, `char`'s and arrays' other methods](decisions/0327-cell-deque-char-methods.md)
- [0328 A `RefCell` counts its borrows, as std's does](decisions/0328-refcell-borrow-counts.md)
- [0329 A `Pin` is its pointer](decisions/0329-pin.md)
- [0330 An `undefined` given to JS is an argument](decisions/0330-undefined-given-to-js.md)
- [0331 A `TypeId` is its type's name, and a `dyn Any` a pair](decisions/0331-dyn-any.md) *(amended by 0660: what unwound is one)*
- [0332 A `MaybeUninit` is its value, or `undefined` before it's written](decisions/0332-maybe-uninit.md)
- [0333 A heap's `append`, `retain`, `drain` and `peek_mut`, and a `Vec`'s `push_mut`](decisions/0333-heap-and-vec-methods.md)
- [0334 A `&mut str` is a cell of the text, written in place](decisions/0334-mut-str.md)
- [0335 A `&mut` of part of a slice is a view of those items](decisions/0335-slice-views.md)
- [0336 A slice's prefixes compare its items by their own `==`](decisions/0336-slice-prefixes-by-eq.md)
- [0337 A slice as arrays of `N`, and arrays as one slice](decisions/0337-slice-as-arrays.md)
- [0338 A slice's `split_off`, and a `&mut` to a shared reference](decisions/0338-slice-split-off.md)
- [0339 `get_disjoint_mut`: a `&mut` to each, checked as std checks them](decisions/0339-get-disjoint-mut.md)
- [0340 A slice of `MaybeUninit`s, and a `const` block of no JS value](decisions/0340-uninit-slices-and-coded-const-blocks.md)
- [0341 A byte slice's `escape_ascii` and `utf8_chunks`](decisions/0341-byte-slice-text.md)
- [0342 An unstable sort is std's own, where its ties can be told apart](decisions/0342-unstable-sort.md)
- [0343 A `&mut` to what's in an `Option` or a `Result`](decisions/0343-option-result-mut.md)
- [0344 `extract_if` is a lazy JS iterator](decisions/0344-extract-if.md)
- [0345 A B-tree's `first_entry` and `last_entry`](decisions/0345-btree-end-entries.md)
- [0346 The next crate binds all of Next.js, and coverage only grows](decisions/0346-next-coverage.md)
- [0347 The react crate's types are measured against @types/react, and only grow](decisions/0347-react-coverage.md)
- [0348 `unwrap_or_default()` of a call is its `??`](decisions/0348-unwrap-or-default-once.md)
- [0349 A body's `js::directive!` is its function's or its closure's](decisions/0349-body-directives.md)
- [0350 Next.js's metadata is typed as Next.js types it, its tagged unions one struct](decisions/0350-next-metadata.md)
- [0351 The node crate is measured against @types/node, and only grows](decisions/0351-node-coverage.md)
- [0352 A name given apart is the name where nothing it'd shadow is read](decisions/0352-reclaimed-names.md)
- [0353 A component's `getInitialProps` is set on it](decisions/0353-get-initial-props.md)
- [0354 An app's `next.config` is Rust](decisions/0354-next-config-in-rust.md) *(superseded)*
- [0355 A default class's statics name its import after the class](decisions/0355-default-class-statics.md)
- [0356 A `const` nothing reads is no statement](decisions/0356-unread-constants.md)
- [0357 A local reuses a name where JS's scopes keep each read its own](decisions/0357-scoped-local-names.md)
- [0358 A number from a `bool` or a `char` is its cast](decisions/0358-numbers-from-bool-and-char.md)
- [0359 next/font: each Google Font a function, generated](decisions/0359-next-font.md)
- [0360 The app's crate is a program: `RUST_JS_APP`, recorded for Cargo](decisions/0360-the-app-is-a-program.md)
- [0361 Node's events are names of their own, typed by their emitters](decisions/0361-node-events.md) *(amended by 0656: `emit` takes what TypeScript's does)*
- [0362 A field's `RefCell` never counted is the value it holds](decisions/0362-refcell-fields-never-counted.md)
- [0363 Children of any node are given by reference](decisions/0363-children-of-any-node.md)
- [0364 Bodies are lowered from their MIR](decisions/0364-mir-lowering.md) *(amended by 0659: tidy's passes are one list of stages, each checked)*
- [0365 JS's Math where Rust's methods differ: `js::math`](decisions/0365-js-math-where-rust-differs.md)
- [0366 A `NodeList` of its items' type, as TypeScript's `NodeListOf`](decisions/0366-nodelist-of-its-items.md)
- [0367 A state's setter is the function it is](decisions/0367-state-setter-is-a-function.md)
- [0368 `util.promisify` of a callback function](decisions/0368-promisify.md)
- [0369 A tuple's parts that act are each held, not an array](decisions/0369-tuple-parts-held-each.md)
- [0370 A loop's exits come first, as a person writes them](decisions/0370-loop-exits-first.md)
- [0371 Tests and names as a person writes them](decisions/0371-tests-and-names-as-written.md)
- [0372 A check whose panic drops is written where it's made](decisions/0372-a-check-that-drops.md)
- [0373 A binding of a JS function is an ordinary function, and a JS type a struct](decisions/0373-one-binding-form.md)
- [0374 An app's Rust is built with Cargo, its one way](decisions/0374-cargo-the-one-build.md)
- [0375 What a build leaves stale is the compiler's to say](decisions/0375-the-compiler-says-what-is-stale.md)
- [0376 A JS name that's a Rust keyword is its raw identifier](decisions/0376-keyword-names-are-raw.md)
- [0377 A format string's arguments are evaluated first, and held where read twice](decisions/0377-a-formatted-value-is-held.md)
- [0378 A Next.js app moves to Rust one file at a time](decisions/0378-one-file-at-a-time.md)
- [0379 A compiler that ran is no change to it](decisions/0379-a-compiler-that-ran-is-no-change.md)
- [0380 The app's config compiles its Rust: `withRustJs(nextConfig)`](decisions/0380-with-rust-js.md) *(superseded by 0412)*
- [0381 The app's Prettier lays out what rust-js writes](decisions/0381-the-apps-prettier-lays-it-out.md)
- [0382 A module has a file for what it has, the root too](decisions/0382-a-module-has-a-file-for-what-it-has.md)
- [0383 A file's leading comments are its JS's](decisions/0383-a-files-leading-comments-are-its-jss.md)
- [0384 A ref a prop holds is @types/react's `Ref<T>`, and an element's props `DetailedHTMLProps`](decisions/0384-a-ref-prop-is-reacts-ref.md)
- [0385 A private type a declaration names is declared, not exported](decisions/0385-a-private-type-a-declaration-names.md)
- [0386 A function that never returns, `-> !`](decisions/0386-a-function-that-never-returns.md)
- [0387 A conversion of a type to itself is the value](decisions/0387-a-conversion-of-a-type-to-itself.md)
- [0388 A boxed slice iterates as its array](decisions/0388-a-boxed-slice-iterates-as-its-array.md)
- [0389 A slice pattern's rest through a `&mut` is a view](decisions/0389-a-mutable-rest-is-a-view.md)
- [0390 Addressable memory is refused, and says so](decisions/0390-addressable-memory-is-refused-as-such.md)
- [0391 What works in Node only is one list, and panics in a browser](decisions/0391-what-works-in-node-only.md)
- [0392 A writer of bytes, `io::Write`, generic and `dyn`](decisions/0392-a-writer-of-bytes.md)
- [0393 An enum's discriminant, `mem::discriminant` and a cast, one way](decisions/0393-an-enums-discriminant.md)
- [0394 A `Deref` bound is given its dictionary, `{ deref }`](decisions/0394-a-deref-bound.md)
- [0395 Function pointers compare by identity](decisions/0395-function-pointers-compare-by-identity.md)
- [0396 A compound assignment in generic code is given its dictionary](decisions/0396-a-compound-assignment-in-generic-code.md)
- [0397 A `RefCell<dyn T>` is its cell, with its `impl`](decisions/0397-a-refcell-of-a-dyn.md)
- [0398 An array some of whose items moved drops what's left, as rustc does](decisions/0398-an-array-some-of-whose-items-moved.md)
- [0399 `hint::unreachable_unchecked()` is unreachable code's throw](decisions/0399-unreachable-unchecked.md)
- [0400 A `Cow` compares and orders as what it borrows](decisions/0400-a-cow-compares-as-what-it-borrows.md)
- [0401 A path compares, orders and keys a map by its components](decisions/0401-a-path-compares-by-its-components.md)
- [0402 A `Cow` keys a map by what it borrows](decisions/0402-a-cow-keys-a-map-by-what-it-borrows.md)
- [0403 A map's value that could look like `None` is given back boxed](decisions/0403-a-map-value-that-looks-like-none.md)
- [0404 A B-tree keyed by value, whose `Ord` is derived](decisions/0404-a-b-tree-keyed-by-value.md)
- [0405 A path's methods go over its components, as Rust's do on Unix](decisions/0405-a-paths-methods.md)
- [0406 A union is the object of the field it holds](decisions/0406-a-union-is-the-field-it-holds.md)
- [0407 A raw pointer is a handle on its place](decisions/0407-a-raw-pointer-is-a-handle-on-its-place.md)
- [0408 Plain data is its bytes](decisions/0408-plain-data-is-its-bytes.md)
- [0409 A struct of a `dyn` last field is the struct, with its `impl`](decisions/0409-a-struct-of-a-dyn-last-field.md)
- [0410 An impl's constant of a function is lowered as code](decisions/0410-an-impls-function-constant-is-code.md)
- [0411 An iterator given as it is is stepped by its dictionary](decisions/0411-an-iterator-given-as-it-is-has-its-dictionary.md)
- [0412 `rust-js build` is its own process](decisions/0412-rust-js-build-is-its-own-process.md)
- [0413 A declaration means the same under every tsconfig](decisions/0413-a-declaration-means-the-same-under-every-tsconfig.md)
- [0414 A library is shared on npm, as ReScript's are](decisions/0414-a-library-on-npm-as-rescripts-are.md)
- [0415 A crate of crates.io's is compiled into the app, where it can be](decisions/0415-a-crate-of-crates-io-in-an-app.md)
- [0416 `RangeBounds` in generic code: a dictionary of the range's bounds](decisions/0416-range-bounds-in-generic-code.md)
- [0417 An allocation is an array of slots](decisions/0417-an-allocation-is-an-array-of-slots.md)
- [0418 A slice's index of a type parameter's is a dictionary](decisions/0418-a-generic-slice-index-is-a-dictionary.md)
- [0419 A `Vec`'s capacity is kept where it's asked](decisions/0419-a-vecs-capacity-kept-where-asked.md)
- [0420 What `Vec::set_len` leaves past a vector's length stays where it was](decisions/0420-what-set-len-leaves-stays.md)
- [0421 A std trait's associated type's drop is its caller's to give](decisions/0421-std-associated-type-drops-are-given.md)
- [0422 An iterator that holds a value is closed to drop it](decisions/0422-an-iterator-holding-a-value-is-closed-to-drop-it.md)
- [0423 hashbrown's maps are std's](decisions/0423-hashbrowns-maps-are-stds.md)
- [0424 indexmap's maps are JS's `Map` and `Set`](decisions/0424-indexmaps-maps-are-js-maps.md)
- [0425 A type's own `Future`, `io::Read`, `BufRead` and `Seek` are allowed, and never lowered](decisions/0425-a-crates-own-future-and-reader-are-allowed-never-lowered.md)
- [0426 A test or a value written out of literals is its answer](decisions/0426-a-literal-test-is-its-answer.md)
- [0427 `#[track_caller]`: a function that reads where it was called is given it](decisions/0427-track-caller.md)
- [0428 A C-variadic function's `...` is JS's rest parameter](decisions/0428-c-variadic-functions.md)
- [0429 A generic or `dyn` double-ended iterator is a JS iterator that knows its back](decisions/0429-a-double-ended-iterator-knows-its-back.md)
- [0430 `mul_add` rounds once](decisions/0430-mul-add-rounds-once.md)
- [0431 std's conversions are one list of what's converted into what](decisions/0431-std-conversions-are-one-list.md)
- [0432 A `return` is written where its value is made](decisions/0432-a-return-is-written-where-its-value-is-made.md)
- [0433 What's written after a loop is after it](decisions/0433-whats-written-after-a-loop-is-after-it.md)
- [0434 A temporary of a variable is the variable, until it changes](decisions/0434-a-temporary-of-a-variable-is-the-variable.md)
- [0435 Rust's world and React's: each its own names, JSX the boundary](decisions/0435-rust-and-react-two-worlds.md)
- [0573 Every Rust name is camelCase in JS](decisions/0573-every-rust-name-is-camel-case-in-js.md) *(amended by 0678: a parameter's leading `_` isn't in its JS name)*
- [0574 What React's scheduler heap asks: a nullable result, an object's `Some`, a runtime that loads anywhere](decisions/0574-what-reacts-scheduler-heap-asks.md)
- [0575 An `if` keeps the `else` it's written with, and a tail `if` of two values is a conditional](decisions/0575-an-if-keeps-its-else-and-a-tail-if-is-a-conditional.md)
- [0576 What a body's integers can be: an index in bounds, an operator that can't wrap, a swap JS's own](decisions/0576-what-a-bodys-integers-can-be.md)
- [0577 A function only calls reach is what its calls give it; a `push` and a `pop` change a length by one](decisions/0577-a-function-only-calls-reach.md)
- [0578 A nullable result is `null` where it's written; an item's place known in bounds is unchecked](decisions/0578-a-nullable-result-where-its-written.md) *(amended by 0638: a nullable function's call is `null` already)*
- [0579 A thread-local is named as its module's variable](decisions/0579-a-thread-local-is-named-as-a-variable.md)
- [0580 A call asks of a cell only what it may borrow](decisions/0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md)
- [0581 A closure called as a statement where it's made is its body](decisions/0581-a-closure-called-as-a-statement-is-its-body.md)
- [0582 `Rc::ptr_eq` of an object is its identity, uncounted](decisions/0582-ptr-eq-of-an-object-is-its-identity.md)
- [0583 A global some hosts lack is an `Option`](decisions/0583-a-global-some-hosts-lack-is-an-option.md) *(superseded by 0597)*
- [0584 A `#[repr(transparent)]` struct is its field](decisions/0584-a-transparent-struct-is-its-field.md)
- [0585 A `RefCell`'s `replace` and `take` hold it for a moment](decisions/0585-a-refcells-replace-holds-it-for-a-moment.md)
- [0586 `#[rust_js::nullable]` is of a field that's an `Option`](decisions/0586-a-nullable-field-is-an-option.md)
- [0587 A function given to `with_borrow` is asked what it borrows](decisions/0587-a-function-given-to-with-borrow-is-asked-as-a-closure.md)
- [0588 A binding may throw by the result it's declared with](decisions/0588-a-binding-throws-by-its-declared-result.md)
- [0589 A `Drop` dropped in one place is its body there](decisions/0589-a-drop-dropped-once-is-its-body-there.md)
- [0590 A `match` of constants tests each by its name, its comments in its arms](decisions/0590-a-match-of-constants-names-them.md)
- [0591 `is_some_and` of an equality with its `Some` is the equality](decisions/0591-is-some-and-of-an-equality-is-the-equality.md)
- [0592 An impl's dictionary is its module's `const`](decisions/0592-a-dictionary-is-its-modules-const.md)
- [0593 `use_state` takes what makes its first state; a lazy argument that does nothing is its value](decisions/0593-a-lazy-argument-is-its-value.md)
- [0594 An overload whose arguments an options dictionary holds is that one](decisions/0594-an-overload-an-options-dictionary-holds-is-it.md)
- [0595 A thread-local `Cell`'s `take` and `replace` are its cell's](decisions/0595-a-thread-local-cells-take-is-its-cells.md)
- [0596 A `pub use` of a module's own item exports it by that name](decisions/0596-a-pub-use-of-a-modules-own-item-exports-it.md)
- [0597 Whether the host has a global is `js::is_defined`'s](decisions/0597-whether-the-host-has-a-global-is-is-defined.md) *(amended by 0645: a static is given as itself)*
- [0598 A temporary is no part of a named variable's pattern](decisions/0598-a-temporary-is-no-part-of-a-names-pattern.md)
- [0599 A result its branches make before it's coerced is returned where made](decisions/0599-a-coerced-result-is-returned-where-made.md)
- [0600 The guide is the site on GitHub Pages](decisions/0600-the-guide-is-the-site.md)
- [0601 A module's variable an `if` makes is a `let` each branch assigns](decisions/0601-a-module-variable-an-if-makes-is-a-let.md) *(amended by 0657: one value each is a conditional)*
- [0602 A binding taken as a `fn` is its JS function, read where it's taken](decisions/0602-a-binding-taken-as-a-fn-is-its-function.md) *(amended by 0667: a variadic one's, of rest arguments as a value)*
- [0603 A comment before a `let` with no JS is the next statement's](decisions/0603-a-comment-before-an-unprinted-let.md)
- [0604 A binding as a value gives its arguments as its call does](decisions/0604-a-bindings-value-gives-what-its-call-gives.md)
- [0605 Whether an arrow only passes on what it's given has one owner](decisions/0605-an-arrow-that-passes-on-has-one-owner.md)
- [0606 A blank line before an item's attributes is before the item](decisions/0606-a-blank-line-before-attributes.md)
- [0607 A variable's field is given a call's value in place](decisions/0607-a-field-is-given-a-call-in-place.md)
- [0608 A `Drop` is its body at each drop](decisions/0608-a-drop-is-its-body-at-each-drop.md)
- [0609 A `let` that shadows a parameter read no more is the parameter assigned](decisions/0609-a-shadowed-parameter-is-assigned.md) *(extended by 0650: any binding of its block)*
- [0610 A function that only calls a module variable's function is that variable](decisions/0610-a-forwarding-function-is-its-variable.md)
- [0611 A default export the macro holds is the function's declaration](decisions/0611-a-default-export-is-where-its-macro-is.md)
- [0612 Two values of any shape are equal by JS's `===`](decisions/0612-unknown-equality-is-strict.md)
- [0613 `&&` and `||` mixed are one value, as written](decisions/0613-and-or-mixed-are-returned-as-written.md)
- [0614 A match arm's pattern of parts is one test of each](decisions/0614-a-patterns-parts-are-one-test.md)
- [0615 `Object.prototype.hasOwnProperty` is a function to call with its object](decisions/0615-has-own-property-the-function.md)
- [0616 An import read only by a `const` is the import by that name](decisions/0616-an-import-read-by-a-const-is-it.md)
- [0617 A parameter is written by its argument](decisions/0617-a-parameter-is-written-by-its-argument.md)
- [0618 `Array.isArray` and `Object.getPrototypeOf` are the js crate's](decisions/0618-array-is-array-and-get-prototype-of.md)
- [0619 A thread-local of a primitive is named as a constant](decisions/0619-a-primitive-thread-local-is-a-constant.md) *(amended by 0634: the registry's symbol is a `const`)*
- [0620 An `Option` a test says is there is written as it is](decisions/0620-an-option-a-test-says-is-there.md)
- [0621 A comment before a macro's opening is its code's; one after a module's last code ends its JS](decisions/0621-comments-around-macros-and-at-a-modules-end.md)
- [0622 A global binding a module names is its `const`](decisions/0622-a-global-binding-a-module-names-is-its-const.md)
- [0624 `js::is_defined` of a path of bindings asks each part](decisions/0624-is-defined-of-a-path-of-bindings.md) *(amended by 0645: a static of a path too)*
- [0625 A default constant is its value; a comment right above the first code is its](decisions/0625-a-default-constant-is-its-value.md)
- [0626 What a library hands a crate is what its items reach](decisions/0626-what-a-library-hands-a-crate.md)
- [0627 A number literal keeps its base and its exponent](decisions/0627-a-number-literal-keeps-its-base-and-exponent.md)
- [0628 A loop after a loop is after it](decisions/0628-a-loop-after-a-loop-is-after-it.md)
- [0629 A `match` of constants is a `switch`](decisions/0629-a-match-of-constants-is-a-switch.md) *(amended by 0634: a `Some`'s constants and the registry's symbols are cases; by 0636: a guarded arm is a case; by 0668: a `match` of what a value is is a `switch (typeof value)`; amended: a `break` only where a case goes on to another)*
- [0630 A comment inside an expression is after the code before it](decisions/0630-a-comment-inside-an-expression.md)
- [0631 A step of one is `++`](decisions/0631-a-step-of-one-is-plus-plus.md)
- [0632 A JS string's `length` is below `u32::MAX`, and a counter below it doesn't wrap](decisions/0632-a-js-strings-length-is-below-u32-max.md) *(amended by 0682: at most `2^31 - 1`, so two add without a wrap)*
- [0682 A JS string's `length` is at most `2^31 - 1`, so two add without a wrap](decisions/0682-a-js-strings-length-is-at-most-int32-max.md)
- [0633 A binding named through a module is read through it](decisions/0633-a-binding-named-through-a-module-is-read-through-it.md)
- [0634 A registered symbol is a `const`, and a `match` of them a `switch`](decisions/0634-a-registered-symbol-is-a-const.md)
- [0635 A `filter` by truth, then a fallback, is `||`](decisions/0635-a-filter-by-truth-then-a-fallback-is-or.md) *(extended by 0646: a chain held, then its fallback; 0651: a `let`, then its fallback)*
- [0636 A guarded arm of a `match` of constants is a case](decisions/0636-a-guarded-arm-is-a-case.md)
- [0637 A `match` of a throwing call's `Result` is `try` and `catch`](decisions/0637-a-match-of-a-throwing-call-is-try-catch.md) *(amended by 0639: whether a call threw is `try`; 0660: `catch_unwind`'s too, and `unwrap_or(d)` returned; amended: an `Ok` arm that leaves, nothing after it, `catch {}`)*
- [0638 A nullable function's call is `null` already](decisions/0638-a-nullable-functions-call-is-null-already.md)
- [0639 `concat_text` of any value is what may throw; whether a call threw is `try`](decisions/0639-concat-text-of-any-value-may-throw.md)
- [0640 A TypeScript rest parameter is a variadic binding of a tuple](decisions/0640-a-rest-parameter-is-a-tuple.md) *(extended by 0667: rest arguments as a value)*
- [0641 A `concat!` of literals is its pieces joined by `+`](decisions/0641-concat-is-its-pieces-joined-by-plus.md)
- [0642 A place found in a string is JS's: `indexOf`, `slice`, `startsWith`](decisions/0642-a-place-found-in-a-string-is-js-units.md)
- [0643 `unwrap()` of a throwing JS call is the call](decisions/0643-unwrap-of-a-throwing-call-is-the-call.md)
- [0644 A function whose result is a throwing call's `Result` is a throwing function](decisions/0644-a-function-of-a-throwing-calls-result-throws.md)
- [0645 `js::is_defined` of a static is given the static itself](decisions/0645-is-defined-of-a-static-is-of-the-static.md) *(amended by 0647: the static isn't read)*
- [0646 A chain held, then its fallback, is one `||`](decisions/0646-a-held-chain-and-its-fallback-is-one-or.md)
- [0647 What `is_defined` names is no read; a test kept by its truth is `&&`](decisions/0647-type-name-as-react-writes-it.md)
- [0648 V8's stack trace API is the node crate's, as @types/node types it](decisions/0648-v8s-stack-trace-api-is-the-node-crates.md) *(amended by 0653: captureStackTrace bound; 0686: the hook's type is named)*
- [0649 `#[inline(never)]` is `/** @noinline */`](decisions/0649-inline-never-is-noinline.md)
- [0650 A `let` that shadows a binding read no more is that binding, assigned](decisions/0650-a-shadow-of-a-binding-read-no-more-is-that-binding.md)
- [0651 A `let`, then its fallback where it's falsy, is `||`](decisions/0651-a-let-then-its-fallback-is-or.md)
- [0652 Consecutive appends to a `String` are one `+=`](decisions/0652-consecutive-appends-are-one.md)
- [0653 A JS function value is a `dyn Fn` of its signature](decisions/0653-a-js-function-value-is-a-dyn-fn-of-its-signature.md)
- [0654 A dictionary's member of any value is a `dyn IntoUnknown`](decisions/0654-a-dictionary-member-of-any-value-is-a-dyn-into-unknown.md)
- [0655 A binding of `any` as a value is a `fn` of `Option<&'static Unknown>`](decisions/0655-an-any-binding-as-a-value-is-a-fn-of-a-static-unknown.md)
- [0656 An event's `emit` arguments are what TypeScript's `emit` takes](decisions/0656-an-events-emit-arguments-are-typescripts-emits.md)
- [0657 An `if` of one value each, a function's too, is a conditional](decisions/0657-an-if-of-one-value-each-is-a-conditional.md) *(amended by 0658: a branch with a comment too)*
- [0658 A comment before a branch's value is before it in the conditional](decisions/0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md)
- [0659 Tidy is a list of stages, each with what it leaves a body with](decisions/0659-tidy-is-a-list-of-stages-with-contracts.md)
- [0660 `catch_unwind` catches what JS throws, a panic among it](decisions/0660-catch-unwind-catches-what-js-throws.md)
- [0664 A function's last slice marked `#[rust_js::rest]` is JS's rest parameter](decisions/0664-a-functions-rest-slice-is-jss-rest-parameter.md)
- [0666 The js crate binds `Object`'s reflection, a descriptor a dictionary](decisions/0666-the-js-crate-binds-objects-reflection.md)
- [0667 `console` is a value, and a `fn` of rest arguments is the JS function](decisions/0667-console-is-a-value-and-rest-arguments-are-one.md)
- [0668 A `match` of what a value is is a `switch` of its `typeof`](decisions/0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md) *(amended by 0676: `undefined` is `case "undefined"`, and `null` in `case "object"`)*
- [0673 A function is a JS value of any shape, whose property `js::set` writes](decisions/0673-a-function-is-a-value-of-any-shape.md)
- [0674 A struct literal's object lists its fields in the order written](decisions/0674-a-struct-literals-fields-are-in-the-order-written.md)
- [0623 A copy std makes of a `Copy` value is a copy](decisions/0623-a-copy-std-makes-is-a-copy.md)
- [0669 An adapter steps its source as std's does](decisions/0669-an-adapter-steps-its-source-as-stds-does.md) *(amended by 0687: and gives the crate's own `fold` and `nth` on)*
- [0670 A tree's writes have one owner, and its walks one definition](decisions/0670-a-trees-writes-have-one-owner.md)
- [0671 A nested body is lowered in one call](decisions/0671-a-nested-body-is-lowered-in-one-call.md)
- [0676 `js::Kind` tells `undefined` and `null`](decisions/0676-kind-tells-undefined-and-null.md)
- [0677 A weak collection's key is what `js::Kind` holds](decisions/0677-a-weak-key-is-what-kind-holds.md)
- [0678 A parameter's leading `_` isn't in its JS name](decisions/0678-a-parameters-underscore-isnt-its-js-name.md)
- [0683 An `if` keeps its test and branches as written, and an `else if` stays a chain](decisions/0683-an-if-is-as-written-and-an-else-if-stays-a-chain.md)
- [0684 A `continue` the program wrote is where it wrote it, with its comment](decisions/0684-a-continue-the-program-wrote-is-where-it-wrote-it.md)
- [0685 A `let` a loop's body declares is in it, set in a block MIR made](decisions/0685-a-let-a-loops-body-declares-is-in-it.md)
- [0686 The stack trace hook's type is named, `js_error::PrepareStackTrace<R>`](decisions/0686-the-stack-trace-hooks-type-is-named.md)
- [0687 A consumer steps its iterator as std's does, through the crate's own `fold` and `nth`](decisions/0687-a-consumer-steps-its-iterator-as-std-does.md)
- [0436 Every `dyn` that's a pair keeps its value's drop, `dyn Send` too](decisions/0436-every-dyn-keeps-its-drop.md)
- [0437 A scope's drops on a panic are one `try { .. } finally { .. }`](decisions/0437-a-scopes-drops-are-one-finally.md)
- [0438 An optional prop is given `Some(..)`, or left out](decisions/0438-an-optional-prop-is-given-some.md)
- [0439 A component is named as JSX names it, and needs no `#[allow]`](decisions/0439-a-component-is-named-as-jsx-names-it.md)
- [0440 A binding needs no `#[allow]`](decisions/0440-a-binding-needs-no-allow.md) *(amended: a descriptor map's fields and a default export's function are used; a plain clippy warns of them)*
- [0441 A `()` given a function value is no argument](decisions/0441-a-unit-is-no-argument.md)
- [0442 A web API that throws for what a program can't prevent is a `Result`](decisions/0442-a-web-api-that-may-throw-is-a-result.md)
- [0443 A value a caller may leave out is an `Option`, in each direction](decisions/0443-a-value-a-caller-may-leave-out-is-an-option.md) *(amended by 0640: a `None` before a rest of none is no argument; 0648: a setter's `None` is written)*
- [0444 A comment is in the JS where its code is, and so is a blank line](decisions/0444-a-comment-is-where-its-code-is.md) *(amended by 0658: a branch's comment is before its value)*
- [0445 Generic code extends and collects into what it's given](decisions/0445-extend-and-collect-in-generic-code.md)
- [0446 A `dyn fmt::Write` is a pair, and `fmt::write` is `write_fmt`](decisions/0446-a-dyn-fmt-write.md)
- [0447 The clock: a `SystemTime` and an `Instant` are their nanoseconds](decisions/0447-the-clock.md)
- [0448 A `transmute` to or from a `#[repr(transparent)]` struct is it made or read](decisions/0448-a-transparent-struct-is-its-field.md)
- [0449 A `PoisonError` gives its guard, and a lock's `Ok` is its guard however it's unwrapped](decisions/0449-a-poison-errors-guard.md)
- [0450 A std holder's `Deref`, `extend` and `collect` are written in place whatever it holds](decisions/0450-a-holders-std-call-in-generic-code.md)
- [0451 A guard in generic code: its `Deref` a dictionary's, its drop given, what's read before it](decisions/0451-a-guard-in-generic-code.md)
- [0452 `{:?}` of a `PhantomData` of a type parameter shows the name its caller gives](decisions/0452-a-phantom-datas-name.md)
- [0453 `{:?}` of a slice's iterator is what's left of it](decisions/0453-a-slice-iterators-debug.md)
- [0454 A `&mut` given as a type parameter's value is a box](decisions/0454-a-mut-given-as-a-type-parameter.md)
- [0455 `nth_back`, `rfold` and `rfind` step from the back](decisions/0455-stepping-from-the-back.md)
- [0456 A generic `ExactSizeIterator` knows its length](decisions/0456-a-generic-exact-size-iterator.md)
- [0457 An `IntoIterator` bound on a reference is given its dictionary](decisions/0457-an-into-iterator-bound-on-a-reference.md)
- [0458 `try_fold` stops at the first `None` or `Err`, leaving the rest to step](decisions/0458-try-fold.md)
- [0459 A hand-written `Serialize` or `Deserialize` is its body, over the JSON codec](decisions/0459-hand-written-serde-impls.md)
- [0460 A callable that writes to a `Formatter` is a writer, and a writer's options are its crate's](decisions/0460-writer-callables.md)
- [0461 A `Formatter` held is a handle on its writer's text, and a `fmt::Result` meets generic code as a `Result`](decisions/0461-a-formatter-held.md)
- [0462 An `io::Error` is a value: its kind, and the error it holds](decisions/0462-io-error-values.md)
- [0463 Hashing is std's: its hashers, and what `Hash` writes to one](decisions/0463-hashing-is-stds.md)
- [0464 A pick drops what it passes over, and a consumed iterator is dropped after](decisions/0464-a-pick-drops-what-it-passes-over.md)
- [0465 Dropping a std adapter is closing it, and what it holds is closed by its own drop](decisions/0465-dropping-an-adapter-is-closing-it.md)
- [0466 A generic map's class is its caller's, a type fact of its key](decisions/0466-a-generic-maps-class-is-its-callers.md)
- [0467 A map's keys with a destructor drop as Rust's do](decisions/0467-a-maps-keys-drop-as-rusts-do.md)
- [0468 A lent adapter leaves its source to its lender, lent once](decisions/0468-a-lent-adapter-leaves-its-source-to-its-lender.md)
- [0469 A function std leaves unrun is dropped there](decisions/0469-a-function-std-leaves-unrun-is-dropped-there.md)
- [0470 A function given by value is dropped where its taker drops it](decisions/0470-a-function-given-by-value-is-dropped-where-its-taker-drops-it.md)
- [0471 A generic unstable sort is sorted as its caller's type picks](decisions/0471-a-generic-unstable-sort-is-its-callers.md)
- [0472 An `Option` an `Option` holds is given as it's held](decisions/0472-an-options-option-is-given-as-its-held.md)
- [0473 `fuse()` is Rust's `Fuse`](decisions/0473-fuse-is-rusts-fuse.md) *(superseded by 0566)*
- [0474 An iterator another bound reads is given as it is](decisions/0474-an-iterator-another-bound-reads-is-given-as-it-is.md)
- [0475 A generic `into_iter()` is the iterator concrete code has](decisions/0475-a-generic-into-iter-is-the-iterator-concrete-code-has.md)
- [0476 A generic `sum` is its caller's](decisions/0476-a-generic-sum-is-its-callers.md)
- [0477 A generic `From::from` is its caller's](decisions/0477-a-generic-from-is-its-callers.md)
- [0478 An associated type of one is given its drop too](decisions/0478-an-associated-type-of-one-is-given-its-drop-too.md)
- [0479 `reduce` is std's](decisions/0479-reduce-is-std-s.md)
- [0480 An associated type steps from its back as a type parameter does](decisions/0480-an-associated-type-steps-from-its-back-as-a-parameter-does.md)
- [0481 An iterator given as it is is written in place](decisions/0481-an-iterator-given-as-it-is-is-written-in-place.md)
- [0482 A slice's pointer cast to what's at the same place is the slice](decisions/0482-a-slices-pointer-cast-to-the-same-place-is-the-slice.md)
- [0483 `BorrowMut` is given a dictionary](decisions/0483-borrow-mut-is-given-a-dictionary.md)
- [0484 A closure its maker gives back carries its drop](decisions/0484-a-returned-closure-carries-its-drop.md)
- [0485 An adapter that holds a function is closed to drop it](decisions/0485-an-adapter-holding-a-function-is-closed-to-drop-it.md)
- [0486 An `Option` of what could look like `None` is changed in place](decisions/0486-an-option-of-a-boxed-payload-is-changed-in-place.md)
- [0487 An iterator given as it is has its extensions' dictionaries](decisions/0487-an-iterator-given-as-it-is-has-its-extensions-dictionaries.md)
- [0488 An iterator bound by an extension alone is bound by `Iterator`](decisions/0488-an-extension-bound-iterator-is-an-iterator.md)
- [0489 A std associated type is given its drop by its own type](decisions/0489-a-yielded-item-is-given-its-drop-by-its-own-type.md)
- [0490 A value std leaves unused is dropped there](decisions/0490-a-value-std-leaves-unused-is-dropped-there.md)
- [0491 `cycle()` keeps a clone of its iterator](decisions/0491-cycle-clones-its-iterator.md)
- [0492 A kept default steps its impl's iterator as the impl does](decisions/0492-a-kept-default-steps-its-impls-iterator.md)
- [0493 A zip of arrays' iterators steps from its back](decisions/0493-a-zip-steps-from-its-back.md)
- [0494 An `Rc` of a type parameter's value is counted where it may drop](decisions/0494-a-generic-rc-is-counted-where-it-may-drop.md)
- [0495 A copied default asks its impl's drops in its impl's terms](decisions/0495-a-copied-default-asks-its-impls-drops-in-its-impls-terms.md)
- [0496 `and` drops what it gives up](decisions/0496-and-drops-what-it-gives-up.md)
- [0497 A std trait's dictionary drops its associated type](decisions/0497-a-std-traits-dictionary-drops-its-associated-type.md)
- [0498 `xor` and `zip` drop what they give up](decisions/0498-xor-and-zip-drop-what-they-give-up.md)
- [0499 `retain_mut` drops what it removes](decisions/0499-retain-mut-drops-what-it-removes.md)
- [0500 A collection made of another keeps its items](decisions/0500-a-collection-made-of-another-keeps-its-items.md)
- [0501 A `splice` owns what it takes out](decisions/0501-a-splice-owns-what-it-takes-out.md)
- [0502 `xor` drops its second value through a panic](decisions/0502-xor-drops-its-second-value-through-a-panic.md)
- [0503 A counted `Rc`'s `borrow()` is its value](decisions/0503-a-counted-rcs-borrow-is-its-value.md)
- [0504 A counted `Rc` of a reference shows what it refers to](decisions/0504-a-counted-rc-of-a-reference-shows-what-it-refers-to.md)
- [0505 An `Rc`'s `as_ref()` is what it points at](decisions/0505-an-rcs-as-ref-is-what-it-points-at.md)
- [0506 `extract_if`, and a map made of items, of values with a destructor](decisions/0506-extract-if-and-made-maps-of-destructors.md)
- [0507 A vector's edits of values with a destructor](decisions/0507-a-vectors-edits-of-destructors.md)
- [0508 No `.map((item) => item)`](decisions/0508-no-identity-maps.md)
- [0509 More of std's calls of values with a destructor](decisions/0509-more-std-calls-of-destructors.md)
- [0510 `flat_map` and `flatten` of what has a destructor](decisions/0510-flat-map-and-flatten-of-destructors.md)
- [0511 The crate's traits that extend `Iterator`](decisions/0511-the-crates-iterator-extension-traits.md)
- [0512 A list of one item left is the item](decisions/0512-a-list-of-one-item-left-is-the-item.md)
- [0513 `iter::repeat_n` and `Peekable::peek_mut`](decisions/0513-repeat-n-and-peek-mut.md)
- [0514 A `Peekable` of what has a destructor](decisions/0514-a-peekable-of-what-has-a-destructor.md) *(superseded in part by 0561)*
- [0515 `map_while`, and the `Some`s an iterator keeps](decisions/0515-map-while-and-kept-somes.md)
- [0516 `rev()` steps what it holds from its back](decisions/0516-rev-steps-what-it-holds-from-its-back.md)
- [0517 `scan` is lazy, and an iterator's combinators one way](decisions/0517-scan-is-lazy-and-iterator-combinators-one-way.md) *(amended by 0669: every adapter steps its source as std's does)*
- [0518 A held `filter_map`, and `$closing` a JS iterator](decisions/0518-filter-map-held-and-closing-iterators.md)
- [0519 A `Cell` or a `OnceCell` of what has a destructor](decisions/0519-a-cell-of-what-has-a-destructor.md)
- [0520 A counted `Rc<dyn T>` is its pair](decisions/0520-a-counted-rc-of-a-dyn-is-its-pair.md)
- [0521 A `dyn Fn` carries its drop](decisions/0521-a-dyn-fn-carries-its-drop.md)
- [0522 A closure cloned apart is made again](decisions/0522-a-closure-cloned-apart-is-made-again.md)
- [0523 Libraries share one counted `Rc` representation](decisions/0523-libraries-share-one-counted-rc.md)
- [0524 A library's `dyn` in its consumer](decisions/0524-a-librarys-dyn-in-its-consumer.md)
- [0525 A cloned `map` or `filter` is made again](decisions/0525-a-cloned-map-is-made-again.md)
- [0526 A `filter` steps from its back](decisions/0526-a-filter-steps-from-its-back.md)
- [0527 An associated iterator steps its impl's parameter](decisions/0527-an-associated-iterator-steps-its-impls-parameter.md)
- [0528 A `fuse` and an `enumerate` from either end, and what an `IntoIterator` gives](decisions/0528-fuse-and-enumerate-from-either-end.md) *(superseded in part by 0566)*
- [0529 A shipped crate is qualified, and memchr's searches are JS's](decisions/0529-a-shipped-memchr-searches-bytes.md)
- [0530 std's iterators of arrays, and a `rev`, clone where they are](decisions/0530-std-iterators-of-arrays-clone-where-they-are.md)
- [0531 `try_rfold` is `try_fold` from the back](decisions/0531-try-rfold.md)
- [0532 A `Debug` builder held in a variable](decisions/0532-a-debug-builder-held-in-a-variable.md)
- [0533 An associated type's `as_mut()` is its dictionary's](decisions/0533-an-associated-types-as-mut-is-its-dictionarys.md)
- [0534 A bound of a sequence's `Index<J>` is its `J: SliceIndex<[T]>`](decisions/0534-a-sequences-index-bound-is-its-slice-index.md)
- [0535 A lazy `map` steps from its back, its source as it is](decisions/0535-a-lazy-map-steps-from-its-back.md)
- [0536 A blanket impl over `?Sized` types keeps its trait's defaults](decisions/0536-a-blanket-unsized-impl-keeps-its-defaults.md)
- [0537 A `Cell` gives an `Option` back as it's kept](decisions/0537-a-cell-gives-its-option-as-its-kept.md)
- [0538 A writer called through a `fn` pointer](decisions/0538-a-writer-called-through-a-fn-pointer.md)
- [0539 A closure writes to its maker's `Formatter`, and `try_for_each`](decisions/0539-a-closure-writes-its-makers-formatter.md)
- [0540 A copied default's types are put in its impl's terms once](decisions/0540-a-copied-defaults-types-in-its-impls-terms-once.md)
- [0541 A copied default's `Self: Sized` is its impl's](decisions/0541-a-defaults-sized-is-its-impls.md)
- [0542 An adapter a struct holds keeps its source and its state](decisions/0542-a-held-adapter-keeps-its-source-and-state.md)
- [0543 A `Peekable` a struct holds keeps what it peeked at](decisions/0543-a-held-peekable-keeps-what-it-peeked-at.md) *(superseded in part by 0561)*
- [0544 A `Cycle` a struct holds keeps both it cycles](decisions/0544-a-held-cycle-keeps-both-it-cycles.md)
- [0545 A map the crate steps from its back is made knowing it](decisions/0545-a-map-stepped-from-its-back-is-made-knowing-it.md)
- [0546 A declaration names a function as its JS does](decisions/0546-a-declaration-names-a-function-as-its-js-does.md)
- [0547 A `copied()` of an iterator with a destructor drops it](decisions/0547-a-copied-iterator-with-a-destructor-is-closed.md)
- [0548 A library's manifest says which type parameters it steps](decisions/0548-a-librarys-manifest-says-which-parameters-it-steps.md)
- [0549 A held `Fuse` is stepped from its back as its use site knows it](decisions/0549-a-held-fuse-steps-what-a-use-site-gives.md) *(superseded by 0566)*
- [0550 A known type's `as_mut()` is the value its `&mut` is](decisions/0550-a-known-as-mut-is-the-value.md)
- [0551 A parameter given as it is, given on where a JS iterator goes, is made one](decisions/0551-self-given-on-where-a-js-iterator-goes.md)
- [0552 An `IntoIterator` whose iterator is held as it is is given as it is](decisions/0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md)
- [0553 `mem::replace` of a field holding an iterator given as it is](decisions/0553-replace-of-a-field-holding-a-given-iterator.md)
- [0554 A filter a type holds, asked to be double-ended, is made knowing its back](decisions/0554-a-filter-a-type-holds-asked-double-ended.md)
- [0555 A library's derived order is field by field](decisions/0555-a-librarys-derived-order-is-field-by-field.md)
- [0556 A `copied()` given back as an `impl Iterator` is what it copies](decisions/0556-a-copied-given-back-as-impl-iterator.md)
- [0557 A `Vec` into an array of its length](decisions/0557-a-vec-into-an-array-of-its-length.md)
- [0558 A byte's escape is its bytes](decisions/0558-a-bytes-escape-is-its-bytes.md)
- [0559 `{:x?}` of an integer is its hex](decisions/0559-debug-hex-of-an-integer.md)
- [0560 A copied default resolves a call with its own bounds](decisions/0560-a-copied-default-resolves-with-its-own-bounds.md)
- [0561 std's bodies are std's own; a recognizer is a proved rewrite to an idiom](decisions/0561-std-bodies.md)
- [0562 A pointer is a place and an address; bytes are plain data's](decisions/0562-a-pointer-is-a-place-and-an-address.md)
- [0563 What's found once of a crate is keyed by the crate's own files](decisions/0563-found-once-is-keyed-by-the-crates-own-files.md) *(superseded by 0573)*
- [0564 A std body's JS is tidied as the app's is: decided tests, once-read temporaries, shared tails](decisions/0564-a-std-body-is-tidied-as-the-apps-are.md)
- [0565 How the generated JS reads is a gate, as what it computes is](decisions/0565-readability-is-a-gate.md)
- [0566 `Fuse` is std's; a specialization is picked by its caller's fact](decisions/0566-fuse-is-stds.md)
- [0567 A corpus case's runs are kept by what they ran](decisions/0567-a-corpus-cases-runs-are-kept-by-what-they-ran.md)
- [0568 A backtrace is disabled unless forced](decisions/0568-a-backtrace-is-disabled-unless-forced.md)
- [0569 A temporary is named as rust-analyzer suggests](decisions/0569-a-temporary-is-named-as-rust-analyzer-suggests.md)
- [0570 A closing source is dropped when it's done](decisions/0570-a-closing-source-is-dropped-when-done.md)
- [0571 `pin!` and a ready future](decisions/0571-pin-macro-and-a-ready-future.md)
- [0050 Snapshots of the generated JS, reviewed as diffs](decisions/0050-snapshots.md)
- [0026 Tests are Rust's `#[test]`, run by `bun test` in happy-dom](decisions/0026-testing.md) *(amended by 0660: a panic is what `$isPanic` says)*
- [0027 Real-browser tests: Playwright Test and Vitest's browser mode, on Bun](decisions/0027-real-browser-tests.md)
- [0032 The playground is written in Rust, compiled by rust-js, a part at a time](decisions/0032-dogfooding-the-playground.md)
- [0044 The playground on React, one component per file](decisions/0044-playground-on-react.md)
- [0045 The playground is a Vite app, with React Compiler and Tailwind](decisions/0045-playground-on-vite.md)

**Architecture boundaries**

- [0073 Named imports for generated modules](decisions/0073-named-module-imports.md)
- [0084 Owned compiler phases and explicit host boundaries](decisions/0084-owned-phases-and-host-boundaries.md)
- [0085 Experimental scalar library linkage](decisions/0085-scalar-library-linkage.md) *(replaced by 0100)*
- [0572 A compiler file is at most 1,500 lines](decisions/0572-a-compiler-file-is-at-most-1500-lines.md)

## Research

Explorations that aren't decisions yet:

- [An in-browser rust-js playground](research/in-browser-playground.md): run rustc's front end + rust-js as WebAssembly
- [Type foundations](research/type-foundations.md): how TypeScript, Scala.js, ReScript and rust-js type JS from the language up to React, and what on par takes

## Adding a decision

Copy the shape of an existing record: **Context → Decision → Why →
Alternatives → Consequences**. Number it next in sequence. If a new decision
replaces an old one, don't delete the old one. Set its status to
`Superseded by NNNN`, so the history of *why* survives. Link it from its
group above, and mark the old one's link *(superseded by NNNN)*:
`test/architecture.test.ts` checks that each is linked, and each
superseded or replaced one marked.
