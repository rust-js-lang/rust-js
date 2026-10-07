# rust-js design docs

rust-js compiles Rust to readable JavaScript. The whole design follows from one
sentence, borrowed from ReScript:

> Keep the language's own front end. Replace only the back end. Where the
> language and JS disagree, pick the JS behavior that keeps the output small
> and readable, and write that choice down.

This folder is where we write those choices down.

For what Rust does under rust-js today, read
[How Rust behaves in rust-js](semantics.md). The decisions below record why,
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
   │                                   │
   ▼                                   │
 rustc analysis (borrow check, lints)  │
   │                                   │
   ├── any error? ──► stop, write nothing  (0006)
   │                                   │
   ▼                                   ▼
 Compilation::Stop               lower.rs: THIR ──► JS AST, with spans  (0008–0015)
 (no rustc codegen)                    │
                                       ▼
                                 prepare.rs: readability preparation
                                       |
                                       v
                                 to_oxc.rs: JS AST ──► oxc AST           (0018)
                                       │
                                       ▼
                                 oxc_codegen ──► one .js + .js.map per module  (0018, 0019)
```

## Code map

[architecture.md](architecture.md) says what's inside `src/lower/`, the
boundaries between the parts, and where a change goes.

| File | Job |
|---|---|
| `src/main.rs` | CLI, rustc callbacks, analysis and the diagnostic gate |
| `src/jsx_syntax.rs`, `src/jsx_syntax/` | Load configured modules and expand JSX tokens into typed Rust before resolution; shared by native and WASM |
| `src/lower/analysis.rs` | Collect named items, imports, trait and mutation facts |
| `src/lower/pipeline.rs` | Orchestrate emission, reachability and owned symbolic module assembly |
| `src/reachability.rs` | Traverse dependencies from caller-selected roots without frontend knowledge |
| `src/link.rs`, `src/names.rs` | Resolve module imports, collision-free aliases and runtime dependencies after lowering |
| `src/lower.rs`, `src/lower/` | Crate facts, function lowering, bindings, representations and JSX semantics |
| `src/lower/serde.rs`, `src/lower/serde/`, `serde/` | `#[serde(..)]` attributes, and JSON written and read as serde_json does; the serde crates, built with the pinned toolchain |
| `src/runtime/from_json.js` | serde_json's reader, ported to JS |
| `src/runtime.rs`, `src/runtime/` | Runtime helpers, each a file of its own, the `@rust-js/runtime` package's (`runtime/`) |
| `src/prepare.rs` | JSX readability preparation after lowering |
| `src/program.rs`, `src/lower/sources.rs` | Owned linked modules and multi-file source origins |
| `src/output.rs`, `src/manifest.rs` | Validated artifact plans and the versioned build result |
| `src/publish.rs` | Native publication, ownership and rollback |
| `tooling/` | Native build adapter, manifest consumers and WASI host publication |
| `wasm/web/compiler-client.js`, `wasm/web/compiler-worker.js` | Browser compiler lifecycle and recovery |
| `src/js.rs` | Our small JS AST; every node carries a Rust span |
| `src/to_oxc.rs` | Converts to oxc's AST, prints, builds the source map |
| `src/format.rs` | Formats the printed JS as oxfmt does, with a crate's options, and moves the source map to match, or to a transform's text of the same program |
| `src/settings.rs`, `src/hooks.rs` | A crate's settings in its `Cargo.toml`, and the transforms and checks it runs on the JS, what they say moved to the Rust (0117) |
| `test/native.rs`, `test/compiler.test.ts` | Differential test: native Rust vs. generated JS |
| `test/emission.test.ts`, `test/diagnostics.test.ts` | Source maps, manifests, output ownership and compiler rejections |
| `test/react.test.ts`, `test/browser.test.ts`, `test/vite.test.ts` | React behavior, browser runners, and real Vite/Fast Refresh |
| `next/`, `next-plugin/`, `test/next.test.ts` | Next.js's bindings, `rust-js-next`, and real Next.js builds and Fast Refresh (0192) |
| `test/sourcemap.ts` | A tiny source map decoder for the tests |

## Decisions

Each record says what we decided, why, what we rejected, and what it costs.

- [0042 Compiler boundaries and build-tool manifest](decisions/0042-compiler-boundaries-and-build-contract.md)

**Foundation**

- [0001 Reuse rustc's front end](decisions/0001-reuse-rustc-front-end.md)
- [0002 Generate JS from THIR, not MIR](decisions/0002-generate-from-thir.md)
- [0003 Pin one nightly and link rustc's internals](decisions/0003-pin-nightly-toolchain.md)
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
- [0012 Panics throw, via runtime helpers emitted on demand](decisions/0012-panics-and-runtime-helpers.md)
- [0013 Fieldless enum variants are strings](decisions/0013-fieldless-enums.md)
- [0033 Enums with fields are ReScript's tagged objects](decisions/0033-enums-with-fields.md)
- [0035 JS that throws is a `Result`; `?` returns early](decisions/0035-results-and-throwing-js.md)
- [0020 Structs are objects, tuples are arrays](decisions/0020-structs-and-tuples.md)
- [0049 Trait dictionaries, generics, and read-only trait objects](decisions/0049-traits-and-generics.md)
- [0047 Methods are an object of functions named after their type](decisions/0047-methods.md)
- [0014 `match` becomes an `if`/`else if` chain](decisions/0014-match-lowering.md)
- [0048 Let chains: each part runs only once the ones before it held](decisions/0048-let-chains.md)
- [0051 `Option<T>` in generic code: boxed only when it looks like `None`](decisions/0051-generic-options.md)
- [0052 The crate's own `Default`, `From` and `Clone`, and the trait ABI kept](decisions/0052-std-trait-impls.md)
- [0053 `==`: JS's `===` or `$eq`, until a hand-written `eq` is in it](decisions/0053-partial-eq.md)
- [0054 `Display`: a `fmt` returns the string it writes](decisions/0054-display.md)
- [0055 The crate's own `Iterator` is a JS iterator](decisions/0055-iterator.md)
- [0056 Indexing: `$index(v, i)` to read, `v[$at(v, i)] = x` to write](decisions/0056-indexing.md)
- [0057 `PartialOrd` and `Ord`: an `Ordering`, and the parts in turn](decisions/0057-ordering.md)
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
- [0066 Text with values in it is a template literal](decisions/0066-template-literals.md)
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
- [0046 `#![rust_js::camel_case]`: a crate's own names, the JS way](decisions/0046-camel-case-crates.md)

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
- [0098 A destructor runs where rustc runs it, in a `finally`](decisions/0098-destructors.md)
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
- [0137 `{:#?}` is pretty Debug, and a writer is told whether](decisions/0137-pretty-debug.md)
- [0138 A string's length, slices and offsets count its UTF-8 bytes](decisions/0138-string-byte-counts.md)
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
- [0162 `AsRef` in generic code: a dictionary, the value itself for std's](decisions/0162-generic-as-ref.md)
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
- [0203 A component's props are as written](decisions/0203-component-props-as-written.md)
- [0204 A flattened field's struct is its parent's props](decisions/0204-flattened-props.md)
- [0205 Flattened structs chain, and the props' own name is theirs](decisions/0205-flattened-chains.md)
- [0206 TypeScript, read and written by TypeScript's own parser and printer](decisions/0206-typescript-module.md)
- [0207 A crate's `.d.ts` is printed by TypeScript](decisions/0207-declarations-printed-by-typescript.md)
- [0208 Each element's attributes, as @types/react types them](decisions/0208-react-attributes.md)
- [0209 A two-arm `match` that's a value is a conditional](decisions/0209-match-conditional.md)
- [0210 A module's declarations import another's types](decisions/0210-declarations-import-module-types.md)
- [0211 `Option::as_deref` of a `String` or a `Vec` is the option](decisions/0211-option-as-deref.md)
- [0212 A props field's default is where JS takes them apart](decisions/0212-props-defaults.md)
- [0213 A component's props are given as one flat list](decisions/0213-props-as-written.md)
- [0214 An untagged enum is its payload: TS's `string | Blob`](decisions/0214-untagged-enums.md)
- [0215 The webapi crate's unions are untagged enums](decisions/0215-webapi-unions.md)
- [0216 A slice's `concat` of parts written out is an array of them, spread](decisions/0216-concat-spread.md)
- [0217 `enumerate()` then a callback's method is the method, given each index](decisions/0217-enumerate-index.md)
- [0218 What JSX holds stays where it's written](decisions/0218-jsx-holds-in-place.md)
- [0219 A sequence a webapi function takes is a slice](decisions/0219-webapi-sequences.md)
- [0220 A JSX tag can be a value, named by a capitalized local](decisions/0220-jsx-tag-values.md)
- [0221 A binding's last slice can be its rest arguments](decisions/0221-variadic-bindings.md)
- [0222 An editor checks the app through rust-js](decisions/0222-editor-check.md)
- [0223 The webapi crate knows each event's type and each tag's element](decisions/0223-webapi-event-and-tag-maps.md)
- [0224 A tag's element reaches its handlers and its `ref`](decisions/0224-typed-intrinsic-elements.md)
- [0225 A JS value of unknown shape is a `js::Unknown`](decisions/0225-unknown-values.md)
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
- [0050 Snapshots of the generated JS, reviewed as diffs](decisions/0050-snapshots.md)
- [0026 Tests are Rust's `#[test]`, run by `bun test` in happy-dom](decisions/0026-testing.md)
- [0027 Real-browser tests: Playwright Test and Vitest's browser mode, on Bun](decisions/0027-real-browser-tests.md)
- [0032 The playground is written in Rust, compiled by rust-js, a part at a time](decisions/0032-dogfooding-the-playground.md)
- [0044 The playground on React, one component per file](decisions/0044-playground-on-react.md)
- [0045 The playground is a Vite app, with React Compiler and Tailwind](decisions/0045-playground-on-vite.md)

## Research

Explorations that aren't decisions yet:

- [An in-browser rust-js playground](research/in-browser-playground.md): run rustc's front end + rust-js as WebAssembly
- [Type foundations](research/type-foundations.md): how TypeScript, Scala.js, ReScript and rust-js type JS from the language up to React, and what on par takes

## Adding a decision

Copy the shape of an existing record: **Context → Decision → Why →
Alternatives → Consequences**. Number it next in sequence. If a new decision
replaces an old one, don't delete the old one. Set its status to
`Superseded by NNNN`, so the history of *why* survives.

**Architecture boundaries**

- [0084 Owned compiler phases and explicit host boundaries](decisions/0084-owned-phases-and-host-boundaries.md)
- [0085 Experimental scalar library linkage](decisions/0085-scalar-library-linkage.md)
