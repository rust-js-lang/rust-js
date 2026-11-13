# What a library tells its consumers

A compiled library is its JS and a contract, `src/library.rs`, in its
manifest: what rustc's metadata doesn't say of how its JS is called. This
page lists every fact of that contract by **what makes it true**, since
that decides what a consumer may rely on:

| Kind | True because | A consumer |
|---|---|---|
| `linkage` | of what the library's source declares, whatever its bodies or its callers do | can't link without it |
| `summary` | of one function, impl or item alone, and what it calls: compositional, as a signature is | gives it at its call, or is checked against it |
| `closed-world` | of what the library's **whole crate** does, or never does | must reach the same answer from another view, or do what the last column says |
| `holds` | it's a list of the rows named | |

A `closed-world` fact is an optimisation that's only sound while every
use is seen. It's allowed where it says what a consumer that differs
does: compile the function itself (`lower/library/compiled_here.rs`,
`lower/instances/needed.rs`), agree, or be refused. The 2026-11-09 audit's
six wrong answers were each such a fact answered apart by a library's
compile and its consumer's ([bug-clusters.md](bug-clusters.md)), so
their agreement is netted: the consumer net and the metamorphic net's
`library` rewrite of every corpus case, each night.

**Adding a fact**: give it a row here, with its kind and, of a
`closed-world` one, what a consumer that differs does
(`test/manifest.test.ts` asks for it), and bump `VERSION`. A new one is
m6-react's to see first (#1462).

## The facts

| Fact | Kind | What it says | A consumer that differs |
|---|---|---|---|
| `Library.version` | linkage | the contract's own version; another is refused | |
| `Library.name` | linkage | the crate's name | |
| `Library.crate_id` | linkage | rustc's identity of the crate, with its name the key it's known by | |
| `Library.crate_hash` | linkage | rustc's hash of the crate its JS was made from, checked against the metadata loaded | |
| `Library.libraries` | linkage | the libraries it was compiled against, whose manifests its consumers must have | |
| `Library.inputs` | linkage | the files it was made of, each with its hash | |
| `Library.package` | linkage | its npm package, where its crate has one | |
| `Library.items` | linkage | each exported item: its key, path, module, export and member, and what its calls give it (`Calling`) | |
| `Library.impls` | linkage | the trait impls whose methods are items | |
| `Library.held` | linkage | the bindings its modules hold as values of their own | |
| `Library.reexported` | linkage | what its modules re-export of its others' items | |
| `Library.modules` | linkage | its modules, each a namespace a consumer may re-export | |
| `Library.types` | linkage | the types its declarations export by name | |
| `Library.keyed` | linkage | its enums told apart by their keys, no tag, by their shape | |
| `Calling.params` | holds | each type parameter it asks anything of: `TypeParam` | |
| `Calling.facts` | holds | `ItemFact` | |
| `TypeParam.index` | summary | which type parameter | |
| `TypeParam.marks` | holds | `ParamFact` | |
| `TypeParam.facts` | summary | the type facts it asks of it, `Size`, `Align`, `RoomSize` and the rest (ADR 0145), which its call gives; of a built receiver, the offset of what std lends from inside it, `OkOffset`, `ErrOffset` and the like, true of the consumer's instance as `Size` is: a call that can't give it, of no layout, is refused (#1478) | |
| `TypeParam.stepped` | summary | the traits it's stepped by as it is, each given a dictionary of | |
| `TypeParam.built` | summary | of a slot past its type parameters', the type built of them whose layout it asks, or where in it std lends from, `Result<T, E>`'s `Ok` (#1478) | |
| `ParamFact::Drop` | summary | it's given what drops a value of it | |
| `ParamFact::Copy` | summary | it's given what copies a value of it | |
| `ParamFact::Clone` | summary | it's given its `Clone` dictionary | |
| `ParamFact::NoDrop` | summary | it takes no destructor: a call that gives one is refused | |
| `ParamFact::ShownPretty` | summary | it's shown pretty, here or by what it's given to: a writer of a type given it takes whether | |
| `ParamFact::ShownHex` | summary | it's shown in hex, as `ShownPretty` | |
| `ParamFact::WrittenInto` | summary | it's written into a writer a piece at a time through its formatting dictionaries, here or by what it's given to: a call gives each with its `fmtInto` (#1652) | |
| `ItemFact::Fails` | summary | it may return `Err(fmt::Error)` | |
| `ItemFact::TakesLocation` | linkage | `#[track_caller]`: its calls give it their location | |
| `ItemFact::Rest` | linkage | `#[rust_js::rest]`: its last parameter is JS's rest, which its calls spread | |
| `ItemFact::Arguments` | summary | it reads JS's `arguments`: its calls give every argument, and no closure that forwards to it is it | |
| `ItemFact::This` | summary | it reads JS's `this`: its calls give it none, and it's used only to be called | |
| `ItemFact::Lazy` | summary | a thread-local made where it's first used, read by calling it | |
| `ItemFact::Variable` | summary | its module's variable, read by its name | |
| `Calling.owned_by_return` | summary | the parameters what it returns alone owns | |
| `Calling.quiet_by_return` | summary | of those, the ones nothing it runs before it returns may call | |
| `Calling.written_arguments` | summary | the `fmt::Arguments` parameters it writes into a writer a piece at a time, itself or by what it gives them to: a call gives each made there as the function that writes into the sink it claims (#1652) | |
| `Library.with_this_arguments` | summary | the parameters of each `with_this` function it makes that reads `arguments` | |
| `Library.gives_argument` | summary | its functions each call of which is what it's given | |
| `Library.displays_value` | summary | its untagged enums whose `Display` writes each variant's value as it is | |
| `Library.partial_cmp_is_cmp` | summary | its `PartialOrd` impls whose `partial_cmp` is `Some(self.cmp(other))` | |
| `Library.identities` | summary | its types equal only to themselves, by their impls | |
| `Library.eq_by_address` | summary | its types whose `==` is their address's | |
| `Library.associated` | summary | each of its traits' associated types whose facts its dictionaries carry, `Size` and `Align` among them (ADR 0145, #1420) | |
| `ParamFact::Unboxed` | closed-world | none of its crate's calls gives it what may look like `None`, so a `Some` of it is the value | compiles its own of the function (ADR 0051) |
| `ParamFact::Counted` | closed-world | its JS counts an `Rc` of it, an `Rc` or `Weak` its MIR holds whose pointee holds it, as its crate's own calls decide | compiles its own of the function where it counts what it gives otherwise, or where two such pointees hold it (#1234) |
| `Calling.nulled_params` | closed-world | each of its crate's calls gives these nullable parameters `null` or a value | gives `?? null` of what may be `undefined`; its use as a value is refused (ADR 0861) |
| `Library.keeps_capacity` | closed-world | whether anything it compiles reads a `Vec`'s capacity, or has a reader of its own, which std's `read_to_end` shows it to (#1297) | keeps its own; compiles its own of a function of a library that keeps none (#1180) |
| `Library.writers_take_options` | closed-world | whether any of its placeholders gives a `Formatter`'s options | compiles its own of each writer that takes fewer (#1026) |
| `Library.debug_hex` | closed-world | whether an integer's `{:?}` its writers write may be hex | compiles its own of each writer (#1026) |
| `Library.dyn_debug_pretty` | closed-world | whether it shows a `dyn Debug` it's given pretty | its writers of a type it makes a `dyn Debug` take that option (#1335) |
| `Library.dyn_debug_hex` | closed-world | as `dyn_debug_pretty`, in hex | as `dyn_debug_pretty` |
| `Library.reads_formatter_options` | closed-world | its functions that read a `Formatter`'s options | compiles its own of each, where its writers take more (#1026) |
| `Library.writes_may_fail` | closed-world | whether any write of its may fail | its `write_all` of the library's writer, or of what's given a dictionary, may fail (#1072, #1392) |
| `Library.makes_dropping_anys` | closed-world | whether it makes a `dyn Any` of a value that drops | its `dyn Any`s carry a drop (ADR 0691) |
| `Library.makes_dropping_errors` | closed-world | whether it makes a `dyn Error` of a value that drops | its `dyn Error`s and `io::Error`s carry a drop (ADR 0462) |
| `Library.makes_dropping_fn_dyns` | closed-world | whether it makes a `dyn Fn` holding a value that drops | a `dyn Fn` it's handed carries a drop (ADR 0626) |
| `Library.makes_dropping_iterators` | closed-world | whether it boxes a `dyn Iterator` of a value that drops | its `Box<dyn Iterator>` carries a drop (ADR 0521) |
| `Library.drops_any` | closed-world | whether any of its own values drops something: a destructor, a counted `Rc`'s count or a guard's borrow; a manifest without it reads as true | gives generic code drops though none of its own values drops, as the library's generic impls' methods it compiles may be given the library's (ADR 0451, amended, #1591) |
| `Library.printing_types` | closed-world | its types one of whose writers may print to stdout as it formats (`Prints::types`) | writes a line that shows one through stdout's line, and its writers that show one through a sink, as of its own types (ADR 1130, amended, #1608) |
| `Library.stopping_types` | closed-world | its types a std value's `Debug` shows piece by piece (`Prints::stopping`) | writes a std value's `Debug` of one through std's builders (ADR 1130, amended, #1608) |
| `Library.stopping_where_caught` | closed-world | its types one of whose writers may stop part-way by a panic, were a consumer to catch it (`Prints::stopping_where_caught`) | one that catches a panic and shows one in stdout's line is refused, naming #1530 (ADR 1130, amended, #1608) |
| `Library.written_into` | summary | its types whose formatting impls' dictionaries carry `fmtInto`, which writes a value into a writer a piece at a time (`WrittenParams::local_adts`): those it writes so, and each it publishes, reachable from outside it, of no type parameters (`Prints::published`) | |
| `Library.dyn_written_into` | summary | whether it writes a `dyn` of a formatting trait it's given into a writer a piece at a time, so a consumer's values made one carry `fmtInto` | |
| `Library.makes_fmt_dyns` | closed-world | whether it makes a `dyn` of a formatting trait whose dictionary may carry no `fmtInto` | one that writes a `dyn` into a writer a piece at a time is refused, naming #1688 (ADR 1135, step 5) |
| `Library.leaves_unended` | closed-world | whether its prints may leave stdout's line, and stderr's, unended | prints each whole line through what the runtime holds, `$print` (ADR 1130, amended, #1599, #1608) |
| `Library.dictionary_consts` | closed-world | a program's: the trait constants its own generic code reads, which its dictionaries carry | one that reads another is refused (#597) |
| `Library.dyns_drop_nothing` | closed-world | a program's: its own traits whose `dyn`s drop nothing | one that makes a `dyn` of what drops is refused (#597) |
| `Library.counted_rcs` | closed-world | what the `Rc`s it counts point at | counts each as it does (ADR 0852) |
| `Library.changed` | closed-world | what its code changes in place, which its JS copies where Rust copies | compiles its own of a function where it may copy what the consumer changes and it doesn't (ADR 0914) |
| `Library.merges_identities` | closed-world | its functions whose JS may give one function where Rust makes two, as it compares none | compiles its own of each, where it compares a function's identity (#1052) |
| `Library.built` | closed-world | the structs its code makes values of, so their fields are plain data | reads those fields in place (ADR 1022) |
| `Library.cloned_closures` | closed-world | each closure it clones, and whether apart | clones one as the library made it (#1202) |
| `Library.ref_cells` | holds | `RefCellFacts` | |
| `RefCellFacts.boxed` | closed-world | its `RefCell`s of an object held as `{ value }`, not the object | holds each so, or is refused (ADR 0863) |
| `RefCellFacts.counted` | closed-world | those some borrow of which it counts | checks each borrow of one (ADR 0863) |
| `RefCellFacts.unchecked` | closed-world | those none of whose borrows can meet another | holds no borrow of one while the library's code runs, or is refused (ADR 0863) |
| `RefCellFacts.handed` | closed-world | those its types hold and its exports take and give | shares each with it as it is (ADR 0863) |
| `Library.fields` | holds | each struct's field its consumers read a fact of: `FieldFact` | |
| `Library.trait_methods` | summary | each of its traits' methods that asks facts of its own slots, and what it asks of each, `[slot, "ConstSize"]`, in the order its JS of the method takes them (ADR 0145, #1612) | a call through the library's exported impl's dictionary gives these, never facts its own analysis of the default's body finds |
| `FieldFact::Quiet` | closed-world | a `RefCell` field none of whose borrows its code checks | lends none, and holds no borrow of one while code that may borrow it runs; lending one is refused (#1054) |
| `FieldFact::RustFunctions` | closed-world | each fn pointer the field holds is a Rust function | its call of one leaves out a trailing `undefined` (ADR 0330) |
