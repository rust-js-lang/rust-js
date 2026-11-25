# Tests at a glance

Changed on 2026-11-25 (#2590, ADR 1179 amended): the crate pairs gain `library_generic_impls_owning_their_parameters`, a library's generic impls, the owners of their type parameters, used by an app.

Changed on 2026-11-25 (#2588, ADR 1104 amended): the metamorphic net runs a hash table's program again up to 128 times, not 8, and gains a test of it; corpus gains `sort_comparisons_as_std_asks`, each sort's comparator calls against native.

Changed on 2026-11-25 (#2577, ADR 0427 amended): 19 corpus cases, each std item that clones what it's given of a borrowed `RefCell` naming std's line against native (`vec_resize_`, `vec_from_elem_`, `slice_fill_`, `slice_to_vec_`, `vec_extend_from_within_`, `vec_deque_resize_`, `write_clone_of_slice_`, `option_mut_cloned_`, and `repeat`'s and `repeat_n`'s steps and clones), and one of a nearer clone placing its own; test/std-sites.test.ts gains a test, each row of `clonedAt` the runtime reads by its key is one the walk writes; test/twin-net.test.ts's measure falls to 204; eleven mutations, one gone with the code it mutated, nine moved with `mir/dispatch/equalities.rs`.

Changed on 2026-11-25 (#2581, ADR 0240 amended): test/crates.test.ts gains a test, a `pub use` of the crate's own static re-exported by its module's JS.

Changed on 2026-11-25 (Campaign 5 step 2c-1, ADR 1202 amended): test/architecture-models.test.ts gains a third test, the compiler reads a row of `$stdSites` through its handle alone; two unit tests, raised.rs' `StdRow`'s rows are those written both ways and each its own, models.rs' each row is the catalog's; test/twin-net.test.ts's ratchet of exports falls to 1,179; seven mutations, two of them new, the stale finds of eleven retyped.

Changed on 2026-11-25 (#2574, ADR 1092 amended): test/architecture-iterators.test.ts gains a test, a fallible collect's helpers made by one maker; test/known/iterator-matrix/2574.txt is gone, its concrete cases native-equal, and 2580.txt lists generic code's.

Changed on 2026-11-25 (#2572, ADR 1090 amended): the consumer net gains #2572's matrix, an item's drop and the source's own drop panicking, of the crate's iterator and a map of it, by each caller and a library's generic code, 15,260 cases; test/known/iterator-matrix/ lists #2574's and #2575's.

Changed on 2026-11-25 (#2570, ADR 1201 amended): test/architecture-stopgaps.test.ts counts 46 stopgap refusals, the nine of Campaign 6's step 7, `Crossing::TypeFact`, among them.

Changed on 2026-11-25 (Campaign 5 step 2a, ADR 1202 amended): test/twin-net.test.ts gains two tests, the runtime's exports the catalog doesn't name yet (1,183, at most) and a whole run keeps a model outside the tables no case imports; its measure falls to 207; each form is named by its row; a unit test of raised.rs, the catalog's rows are written; six mutations, two moved to scripts/mutations/lower/recognition/models/rows.ts.

Changed on 2026-11-25 (Campaign 5 step 1, ADR 1202, ADR 1122 amended): test/architecture-models.test.ts, new, two tests (no second list of the runtime's models); test/twin-net.test.ts reads the catalog `rust-js --std-models` prints, gains its measure (220 models stand for no std item yet, at most), and its bless tests write models.rs; five mutations.

Changed on 2026-11-25 (#1979): test/mutation-reach.test.ts, new, three tests: scripts/mutation-reach.ts' answers, the target it suggests, its line of a mutation and what of the JS it compares.

Changed on 2026-11-25 (#2539, Campaign 6 step 0, ADR 1203, ADR 1199 amended): test/param-form-shadow.test.ts, new, two tests: `ParamForm`'s shadow, `--shadow param-form=<file>`, of each value converted where it crosses into or out of generic code the form taken beside every form that differs there, and of each iterator form asked the bounds that read its value; no output changed. `bun scripts/param-form-shadow.ts` counts the corpus's lines.

Changed on 2026-11-25 (#2485, ADR 1093 amended): test/judged.test.ts, new, three tests: a working tree's tree is its files as they are, its untracked files too, not its ignored; what a run judged is recorded by its tree, the newest kept; the change's mutations not judged on its tree are those it doesn't record. test/ci.test.ts gains a test, check:fast judges the change's mutations as CI's check does, a survivor at its base latent; test/select.test.ts' push phases gain `judged`; seven mutations (scripts/mutations/judged.ts, `check-fast-mutations-without-base`). test/mutations.test.ts gains a test, a test moved to another file is found by its name, and --remap writes where it went in its list (ADR 1200 amended); mutations `test-moved-to-the-first-of-several`, `remap-one-line-only`. Its retirement test is now a retirement checks React's port at its pin before its other controls, which run at once, kept for its tree (ADR 0093 amended); mutations `retire-controls-one-after-another`, `retire-controls-kept-unread`, `retire-controls-kept-of-any-tree`.

Changed on 2026-11-25 (#2567, ADR 0061 amended): corpus gains `generic_bytes_stepped_by_next`, a reader's `bytes()` in generic code stepped by `next()`, held, by a `for` and taken.

Changed on 2026-11-25 (ADR 1118 amended): test/support.test.ts gains a test, the Crater's work a run is using is never removed under it, held before or after the prune lists it, by its cap's bound and the checkouts' budget alike; the crate corpus's test holds its projects by test/kept.ts' `whileHeld`, the one claim; mutations `crater-held-listed`, `crater-removed-unclaimed`, `claimed-thing-used-unheld` and `claim-making-listed`, where `crate-project-removed-unclaimed` and `crate-project-measured-unheld` are now `kept-app-removed-unclaimed` and `claimed-thing-used-unheld`.

Changed on 2026-11-25 (#1979): corpus gains `insert_of_none_at_an_index_known_in_bounds`, a `None` inserted where the body's ranges show its index in bounds, `v.splice(at, 0, undefined)`; test/lowering-strict-switches.test.ts' chain-stays test gains a chain of a getter-answered global.

Changed on 2026-11-25 (ADR 1200 amended): test/mutations.test.ts' check of each mutation's test pattern is each mutation's tests, a file there and a test it names, refused as a list of what to run (`refuseUnmatched`), of a file named by templates by Bun's listing where the change touches it (test/test-names.ts, new); a test of the read and the listing; three mutations (scripts/mutations/test-names.ts).

Changed on 2026-11-25 (#2485, ADR 1154 amended): test/source-mutations.test.ts gains a test, a source workspace's files are the working tree's, its untracked files too, not its ignored, its deleted nor a nested checkout; three mutations.

Changed on 2026-11-25 (#2564, ADR 1092 amended): corpus gains `fallible_collect_drops_what_it_collected_as_its_close_panics`, an `Option` or `Result` collect stopped early whose close panics dropping what it collected, then the `Err`; test/known/metamorphic/2564.txt is gone, its `loud-drop` rewrite native-equal.

Changed on 2026-11-25 (Campaign 5 step 1, ADR 1202, ADR 1122 amended): test/architecture-models.test.ts, new, one test (no second list of the runtime's models); test/twin-net.test.ts reads the catalog `rust-js --std-models` prints, gains its measure (220 models stand for no std item yet, at most), and its bless tests write models.rs; five mutations.

Changed on 2026-11-25 (#2561, ADR 0570 amended): corpus gains `generic_fallible_collect_drops_its_iterator_once`, generic code's `collect()` into an `Option` or a `Result` dropping the iterator it's given once; test/known/metamorphic/2564.txt lists its `loud-drop` rewrite under #2564.

Changed on 2026-11-25 (#2490 phase 2, ADR 1199 amended): test/cell-form-shadow.test.ts gains two tests, eight in all: a binding run while a guard's held, passed by Tu's exceptions 39 and 44, each site named, or by what it's given (a builtin's `Map`); a `try_borrow` nothing can contest, another field's borrow, a type parameter's and an `Rc`'s drop, a thread-local's `with_borrow_mut` and a generic body's loud type; its whole-use test gains proofs A and B and the `momentary` verdict; 23 mutations of `cell_form.rs`, `cell_borrows.rs` and `cell_borrows/unproven.rs`.

Changed on 2026-11-25 (#2562, ADR 1118 amended): test/support.test.ts gains a test, a crate project being measured is never removed under its run, claimed before or after the prune lists it, by its cap's bound and the checkouts' budget alike; four mutations.

Changed on 2026-11-25 (#2543, ADRs 0562, 1022 and 1120 amended): corpus gains `option_payload_address_after_a_generic_walk`, `option_map_subject_written_through_a_kept_mut` and `hashed_map_clone_hashes_no_key`; test/known/metamorphic/2543.txt is removed; mutations `ptr-offset-unrefined`, `subject-handle-unread`, `hashed-clone-rehashed`, `clone-into-rehashed`, `assign-rehashed` and `apart-hashed-plain`.

Changed on 2026-11-25 (#2558): test/support.test.ts gains a test, a claim whose process is gone is taken over only by the one process taking it, in-process, which judges `claim-taken-over-unlocked` where the racers' child processes judged it by luck; mutation `claim-taking-left-by-the-dead-kept`.

Changed on 2026-11-25 (ADR 1122 amended): test/twin-net.test.ts gains three tests, a gap fails a run, drift fails none and its bless writes the lists, and `BLESS=1 bun test/twin-net.ts helpers` blesses a whole run that passed alone; six mutations (scripts/mutations/twin-net.ts).

Changed on 2026-11-25 (#2539, #2557, ADR 1201): test/architecture-stopgaps.test.ts, new, four tests: each of Campaign 6's stopgap cells, `refusal::Crossing`, is mapped once to an issue of its own and named by a refusal, 33 counted; a stopgap is made by the refusal API alone; a cell whose issue is closed is named. The issue-labels workflow runs scripts/stopgap-refusals.ts as an issue closes and daily. test/diagnostics.test.ts' three `extend` rows assert the refusal's words as `refuse` says them.
Changed on 2026-11-25 (ADR 1200): test/case-lists.test.ts, new, two tests: a list of what to run naming a path or a misspelt name fails, naming each, the corpus's `RUST_JS_CASES=i/n:<file>` among them, which the metamorphic net, tidy's stages, the iterator forms and the determinism net read too; five mutations.

Changed on 2026-11-25 (#2538, ADR 1184 amended): test/binaries.test.ts gains a test, a panic with its output a file runs nothing `main` queued, against native, on Node and Bun; mutation `file-stream-waited-for`.

Changed on 2026-11-25 (#2532, ADR 0454 amended): test/diagnostics.test.ts gains two rows, a `&mut` to an object in an `Rc` or a `Box` given to generic code by an impl over `&mut T`, refused; mutations `impl-self-mut-object-unasked`, `impl-self-own-mut-refused`; test/known/metamorphic/2532.txt removed, its 2 rows now refused.

Changed on 2026-11-24 (#2530, ADR 0427 amended): corpus gains `tracked_fn_returned_names_fn_calls_line`, `generic_clone_returned_names_fn_calls_line` and `tracked_fn_returned_by_a_pub_fn_refused`; three mutations.

Changed on 2026-11-24 (#2531, ADR 0270 amended): corpus gains `thread_local_object_borrowed_from_another_module`, and the crate pairs `thread_local_object_of_a_library_borrowed`; `exported_cells`' snapshot loses its two counted borrows of another module's thread-local; mutation `quiet-local-counted-in-another-module` replaces `quiet-with-borrow-of-another-module`; test/known/metamorphic/2531.txt removed, its 2 rows run as Rust does; the corpus row recounted, 3,028 (`ls test/corpus/*.rs | wc -l`).

Changed on 2026-11-24 (#2522, ADR 0427 amended): corpus gains `generic_clone_made_a_value_names_call_muts_line`, `fn_mut_moved_into_option_map_names_call_onces_line` and `clone_held_in_a_struct_given_whole_names_call_onces_line`; test/iterator-forms.test.ts' `differs` loses its three #2522 rows and test/known/metamorphic/2522.txt is removed; seven mutations.

Changed on 2026-11-24 (#2481, ADR 1104 amended): the metamorphic net gains two nightly-only rewrites, `observe-borrows` and `observe-lens`, and four tests (each rewrite's probes, a push's sample runs no nightly-only rewrite, and a rewrite whose every variant is refused judged nothing); test/known/metamorphic/ gains 2531.txt (2 rows), 2532.txt (2) and 2533.txt (126), and 2522.txt six; their sweep, the corpus's `RefCell` cases in both builds, 2 min a part in 10 parts on the Mac mini.

Changed on 2026-11-24 (#2529, ADR 0863 amended): corpus gains `generic_cloned_of_a_held_refcell_panics` (`located-otherwise` #2522); mutations `cell-parameter-unborrowed` and `cell-parameter-its-own-cell`.

Changed on 2026-11-24 (#2494, ADR 0979 amended): mutation `nested-alias-kept`; four corpus snapshots lose a closure's alias of a dictionary its body declares.

Changed on 2026-11-24 (#2490, ADR 1199): test/cell-form-shadow.test.ts, new, six tests: `CellForm`'s shadow, `--shadow cell-form=<file>`, of each `RefCell` borrow the form the compile takes beside the proof's from MIR, alone and with the crate's bodies' borrow summaries, and what defeated it: a borrow for a moment of std's method, a guard held across the crate's function the summaries prove asks nothing, its range ended where it's dropped, a guard held across what borrows its cell again or another borrow of it, a `try_borrow`, and a field a derive or a function is given whole.

Changed on 2026-11-24 (#2490, ADR 1199): one shadow flag, `--shadow <kind>[,<kind>]=<file>`: test/cargo.test.ts gains a test, `RUST_JS_SHADOW` gives a Cargo build's crate the lines `--shadow` gives it; test/drops-shadow.test.ts gains a test, the old shadow flags and an unknown kind refused; the intervals' and drops' shadows' tests read their lines by kind.

Changed on 2026-11-24 (#2509 step 2, ADR 0427 amended): corpus gains ten cases, seven of an item std's method clones of a borrowed `RefCell` (a `cloned()`'s `count`, `fold`, `next`, `nth` and `any`, an `Option<&T>`'s `cloned`, `extend_from_slice`) and three `located-otherwise` against #2509 (a `cloned()` collected, looped over, and asked `next` then `count`); test/known/metamorphic/2509.txt (8 rewrites) and 2522.txt (53), and three of test/iterator-forms.test.ts' `differs` against #2522; test/std-cloned-at.rs gains three rows (`Cloned`'s `nth`, `max_by`, `min_by`); the corpus row recounted, 3,022 (`ls test/corpus/*.rs | wc -l`).

Changed on 2026-11-24 (#2509, ADR 0427 amended): test/std-cloned-at.test.ts, new, one test: the native oracle of where std's clones raise a borrowed `RefCell`'s panic, test/std-cloned-at.rs run natively, kept as crates/rust-js-lower/src/recognition/compiled_std/cloned_at_oracle.txt; test/architecture-panics.test.ts gains a test, a std type's clone site is read of the impl rustc resolves, not a list; corpus gains four cases, a `HashMap`'s, a `BTreeMap`'s and a `VecDeque`'s clone of a borrowed `RefCell`, and one `located-otherwise` by the guide's difference 65; a case's `located-otherwise` may name a difference as well as an issue.

Changed on 2026-11-24 (#2489, ADR 1184 amended): test/architecture-panics.test.ts, new, two tests: a panic's `Error` is made in one place, moved from test/architecture.test.ts by concept, and each panic the runtime raises passes its site or says in the comment above it why it has none, `// no site: ..`; corpus gains `rc_slice_collect_past_a_layout_panics_at_its_unwrap` and `arc_slice_collect_by_generic_code_past_a_layout`, each against native Rust's hook.

Changed on 2026-11-24 (#2447, ADR 1198): every corpus case is built as a binary, `--crate-type bin`, its snapshot the binary's JS, run as a process, `node case.js`, against native Rust's binary of the file: status, stdout, and stderr with std's hook of the panic that ends `main` (`compared`); the wrapper, the program build, compiled for every case and run where its JS isn't the binary's, the library build as before. Every net through test/programs.ts follows (metamorphic, tidy's stages, the matrices, fuzz, the object model, rustlantis). test/programs.test.ts gains three tests, a panic's hook compared, an abort's forced frames left out, a run's own directory; test/corpus.test.ts's location control is one, a `located-otherwise` case raised where native's is. On the 12-core Mac mini, the whole corpus in 6 parts: warm 5–7 s as before; every compile run (a compiler change) 58–59 s against 41–42 s, the binary's compile a session of its own; the first run after it, about 2,900 native binaries built and first checked by macOS, 335 s, once.

Changed on 2026-11-24 (#2450, #2451, ADR 0863 amended): test/binaries.test.ts gains three tests, `let_else_guard_released.rs`, `refcell_held_while_asked.rs` and `refcell_try_borrow_in_a_closure_while_held.rs` built as binaries and run against native Rust, a `try_borrow_mut()` a closure or a `Drop` asks while a borrow of its cell is held finding it borrowed, which a library's build judges by the cell's type and a binary's by its flow; and the corpus gains the last.

Changed on 2026-11-24 (#2112, Change A step 2b′): the paths table gains two paths, a library's generic function given the adapter, as itself and in a `Box`, the library compiled apart from its own JS and manifest as a dependency is, 5,877 cases in 99 programs, still all of it per push of the iterators' code; its first run found #2419.

Changed on 2026-11-23 (#2392, ADR 1189): test/declarations-readonly.test.ts, new, one test, type-checked: a shared borrow of an array declared `readonly T[]` as a parameter, a result and a field, a `readonly` array accepted where Rust reads and refused where it takes `&mut`; five declarations tests' expectations say `readonly` where they declare such a borrow.

Changed on 2026-11-23 (ADRs 0214, 0284): test/serde-attributes.test.ts gains a test, a discriminated union's `otherwise` variant serde reads, against native serde, and its JS value the object it holds.

Changed on 2026-11-23 (#2388, ADR 0214 amended): test/serde-attributes.test.ts gains a test, a `#[rust_js::untagged]` enum serde reads and writes against native serde, by each of its representations, and its JS values its payloads; test/diagnostics.test.ts loses its case of the refusal, now compiled.

Changed on 2026-11-23 (#2382, #2383): test/serde-attributes.test.ts gains a test, a field of its own reader that isn't there against native serde: `deserialize_with` of a function of the crate's and of `Option::deserialize` and `Vec::deserialize`, beside `#[serde(default)]`, `with = "module"`, `serialize_with` beside `skip_serializing_if` and of `Option::serialize`, and an adjacently tagged newtype variant's reader given its missing content.

Changed on 2026-11-23 (#2379, ADR 1188): test/serde-path.test.ts, new, four tests: serde_path_to_error's read against native serde_path_to_error, every rule of where in the value it places a mistake; its `Error` shown padded, as `{:x?}`, into a writer and as a `dyn Error`, and a type's own `deserialize` of serde_json's reader of text; what's refused of either; and its `Error` and `Path` declared `@rust-js/std`'s opaque `JsonPathError` and `JsonPath`.

Changed on 2026-11-23 (#2325, ADR 0083 amended): test/binaries.test.ts gains a test, a registry crate's panic names its own line against native Rust, serde_json's three `IndexMut` panics, hashbrown's own map's and indexmap's `get_disjoint_mut`; test/shipped-crates.test.ts gains one, serde_json's sites noted by Cargo's check through rust-js and given; test/std-sites.test.ts gains one, std's build's libraries linked once; rust-js-program's unit tests gain `library::registry_sites`' finder.

Changed on 2026-11-23 (#2334, ADR 1186): test/helper-calls.test.ts, one test, holds each file's reads of a helper call by its callee's name, `helper_name()`, at most and exactly test/helper-shapes-known.txt's, a ratchet migrating a file onto `helper_call_of` lowers; rust-js-program's unit tests gain `runtime::signatures`' parse of a parameter list and `js::helper_call`'s, each name a call is read by one its helper declares, of every compiler source.

Changed on 2026-11-23 (#2324, ADR 0427 amended): test/binaries.test.ts gains two tests, a panic std's impl raises at its caller through a dictionary (integers' `/` and `%`, `clamp` of integers, a `char`, a `String`'s default, a generic `SliceIndex`'s) and a `Repeat`'s `count` and `last`, directly, through generic code, nested in another count and after one caught; test/runtime-marks.test.ts, ten tests, a library's `Repeat` counted by its app; the corpus's `own_ne_max_min_clamp` and `repeat_drops_what_it_holds` compare their locations again.

Changed on 2026-11-23 (#2343): test/std-sites.test.ts gains two tests: no committed JS rust-js wrote, std/, the runtime, the corpus's and the snapshots', names a machine's own copy of std's source or its home; and std's bodies, compiled into a fixture, name std's files `/rustc/<commit>/library/..`.

Changed on 2026-11-23 (#2333, ADR 1184 amended): test/runtime-marks.test.ts, nine tests: the runtime makes no mark by `Symbol(..)`, all ten in runtime/marks.js, and each mark of a value one copy makes and another reads, an `Rc`'s, an array iterator's, a part's, chunks', a closure's clone's, an `IterMut` item's, a sliced `IntoIter`'s, run against native Rust with a library on a copy of the runtime of its own.

Changed on 2026-11-22 (#2292, ADR 1184): test/binaries.test.ts, thirteen tests, each a binary's run against native Rust's on Node and Bun read as one stream; test/std-sites.test.ts holds `bun run std`'s `$stdSites`; test/runtime-marks.test.ts, two tests, holds the runtime's marks every copy of it reads, and a library's copy's panic read by its app's against native Rust's. Every net that runs a program against native Rust (test/programs.ts) now compares where a panic was raised, `{ panic, at }`, but where a case's `//@ located-otherwise: #<issue>` says it waits (three cases, #2324 and #2335); the corpus's uncached run, 2 min 31 s in twelve parts here, against 2 min 33 s before. The inline JS tests' compiles leave their fixture's directory out of the files they name (test/child.ts `located`), so a location reads `lib.rs:6:5`.

Changed on 2026-11-20 (#2112, Change A step 2b's paths): the paths table gains six paths, generic code given its iterator through a trait (of the adapter, a `Box` of it, a `&mut` of it) and the wrappers nested (`Box<Box<A>>`, `&mut Box<A>`, `Box<J>` of a type parameter), 5,125 cases, still all of them per push of the iterators' code.

Changed on 2026-11-20 (#2112, Change A step 2a): the paths table, `ITERATOR_TABLE=paths` of test/iterator-matrix.test.ts (test/iterator-paths.ts), 2,869 cases, all of them in each push of the iterators' code (a step of `nets-iterators`) and in `check:fast`.

Changed on 2026-11-19 (#2162, ADR 1175): test/architecture-owners.test.ts, nine tests in push's fast phase, holds the table of semantic owners complete for the front end and each row's primitive its owners' alone; the old check that the table's modules and APIs are there moved into it from test/architecture.test.ts.

Changed on 2026-11-19: `test/ci.test.ts`'s tests of how the suite and the mutations are split into shards, and how many jobs run at once, are `test/shards.test.ts`, fourteen tests, split by concept near the cap.

Changed on 2026-11-19 (#2058): corpus `own_write_fmt_through_generic_dyn_and_macro`, and it and `own_write_fmt_given_a_held_arguments` in `tidy-stages`' few cases.

Changed on 2026-11-19 (#2112, Change C's third step): test/one-none-exits.test.ts, seven tests, holds what an exit that gives `None` as `undefined` gives as it is, by MIR.

Changed on 2026-11-19 (#2112, Change C's second step): a unit test, `lower::mir::none_forms::tests`, holds that a local made `null` is never read as `undefined` alone.

Changed on 2026-11-19 (#2135): test/builtins-runs-nothing.test.ts gains three tests, of a generic read under its own body's guard.

Changed on 2026-11-19 (#2115): test/builtins-runs-nothing.test.ts gains three tests, of a tuple struct its constructor builds.

Changed on 2026-11-19 (#2116): test/at-hand-returns.test.ts, six tests, holds what a function returns, and what passes a value on, at hand where what it's made of is.

Changed on 2026-11-19 (#2114): test/at-hand-slots.test.ts, eight tests, holds what a thread-local the crate alone reaches holds, and a local cell its closures share, at hand where every write of it is.

Changed on 2026-11-19 (#2103): every test of what TypeScript makes of generated declarations checks it by test/type-checked.ts's `typeChecked`, its settings its parameters, where 33 test files ran `tsc` their own way; test/test-helpers.test.ts holds them to it.

Changed on 2026-11-18 (#1971's cause): test/std-fresh.test.ts, one test, holds std/'s freshness check, which each push's lint job runs (`bun scripts/std-fresh.ts`), to naming what differs in std/ alone.

Changed on 2026-11-18 (#2082, #2106, #2110, #2109, #2111, #2113): test/builtins-runs-nothing.test.ts, thirty tests, holds the js crate's builtins that run none of the program's code, and property reads that run nothing only of what a builtin made: a memo's borrow over a regex, a locale's own or an `exec` match reads plain; over a callback, a JS caller's locale or a map's kept value it's checked.

Changed on 2026-11-18: the ES class suite checks static methods, declarations, and JavaScript's implicit constructor on a fieldless class.

Changed on 2026-11-18: the comment placement suite checks that a wrapped template initializer keeps its annotation outside the string.

Changed on 2026-11-18: the None regression holds a return of a locally written null beside undefined and a present value in a single variable. It checks JavaScript behavior and TypeScript declarations. It also checks a binding whose outer option is undefined-only but whose object contains optional fields.

Changed on 2026-11-18: the ES class regression verifies that an ordinary Default implementation compiles and gives instances through both constructors and Rust trait calls.

Changed on 2026-11-18: the nullable proof regression checks an option filled on its missing branch, including a local initialized by a lookup. Paths that skip the fill or mutate the option through a borrow still panic on unwrap.

Changed on 2026-11-18: the declaration suite checks relative foreign types in a nested module and resolves them from a TypeScript consumer.

Changed on 2026-11-18: the map binding regression checks strict undefined comparisons and the behavior of missing, null, and object values in native Map and WeakMap.

Changed on 2026-11-18: the constructor binding regression checks direct JavaScript construction from a property read, getter order, and TypeError for invalid targets.

Changed on 2026-11-18: the argument regression checks JavaScript default syntax, arity, null, undefined, and fallbacks that read later arguments or have effects.

Changed on 2026-11-18: the constructor counter regression holds a postfix increment beside its field assignment and preserves the old value when another field needs it; it also holds a function's own read-then-step as two statements, React's `act`, and a constructor of the `get`/`set` form whose old value is read twice.

Changed on 2026-11-18: the constructor field-comment regression holds the original JSDoc beside each emitted assignment in an ES class.

Changed on 2026-11-18 (#2070): test/on-load-names.test.ts, two tests, holds
an `on_load!` body's top-level local beside a function and an import of its
word, given apart and run.

Changed on 2026-11-18 (#2067, #2072): corpus case
`impl_iterator_own_nth_skipped_in_a_flat_map.rs` holds an own `nth` behind
an `impl Iterator` asked as std's `try_fold` asks it; test/cross-crate-iterators.test.ts
a library's; test/architecture-iterators.test.ts holds whether an iterator
is written in Rust to `is_user_iterator` alone.

Changed on 2026-11-18 (#2052): corpus case
`map_as_it_is_boxed_beside_an_enumeration_from_its_back.rs` holds an
enumeration stepped from its back over a map made knowing its back alone.

Changed on 2026-11-18 (#2051, #2056): test/iterator-steps.test.ts, three
tests, holds each runtime wrapper that passes its source's steps on to
reaching every step of `$stepNames`, and the two flattens to the same steps;
corpus cases `generic_code_steps_adapters_as_std.rs` and
`collected_in_place_past_a_capturing_filter_whose_rejected_drop_panics.rs`.

Changed on 2026-11-18 (#1944's regression): corpus case
`guard_lent_to_a_closure_proves_remove.rs` holds a `RefCell` guard lent
shared to a closure, its `remove(0)` past a push proved, and a `Cell` field
the closure changes through it, its index checked.

Changed on 2026-11-18 (#1908's scope): corpus case
`option_subject_of_a_cells_field.rs` holds an `Option` combinator's subject
read through a captured `Rc<RefCell<_>>`'s guard, read again where nothing
runs between, as upstream writes it, and `nullable_field_of_a_mapped_option.rs`
React DOM's `getParent` step.

Changed on 2026-11-18 (#1904's markers): test/object-markers.test.ts, two
tests, holds every binding crate's `Classed` type to what an object's
binding is, `Defined` and `WeakKey`; test/intl-locale.test.ts's sixth, an
`Intl` object a value of any shape and a weak key.

Changed on 2026-11-18 (#1904): test/intl-locale.test.ts, five tests, holds
`Intl.Locale` as it's written from TypeScript's library: Rust callers of
each member on Bun, a strict TypeScript caller, and the generator's refusal
of a form it doesn't bind.

Changed on 2026-11-18 (#1903): test/definitions.test.ts, four tests, holds
the definitions update's release policy (stable first, a prerelease only for
a capability that names its test), one TypeScript pin for the parser and the
library, the update's whole run against fakes, and the capability the pinned
beta is kept for; test/builtins-coverage.test.ts adds two, every unbound
member named with its issue (builtins/gaps.txt); test/typescript.test.ts a
library selection.

How these suites find wrong answers, and what each has found, is in
[docs/finding-wrong-answers.md](docs/finding-wrong-answers.md).

Corpus recount on 2026-11-15: 2,559 Rust programs. The new
`enum_variants_of_the_same_fields_copied_and_compared_once.rs` covers a
tagged enum's variants of the same fields cloned and compared once, one
test of them all, and test/type-depth.test.ts an enum chain of every
derive 10, 20 and 40 deep, each build, its JS growing with its depth, not as its variants to its power (#1821).
The new
`glue_dropped_items_passed_from_a_vecs_back.rs` covers items whose drop is
their glue passed over from a `Vec`'s back, each dropped front to back
(#1854), and `reversed_map_stepped_from_its_back.rs` a `rev()` stepped
from its back by each back step (#1855). The new
`ordering_predicates.rs` covers all six predicates, function values, receiver
evaluation and UTF-16 ordering against native Rust in both builds (#1827).
The new `refcell_guard_bounds.rs` covers each way a `RefCell`'s guard's
length is forgotten, each index checked as Rust panics (#1815).
The new `map_key_changed_between_test_and_index.rs` covers keys whose lookup a
call may change, each `m[k]` checked as Rust panics (#1844).
The new `table_rows_resized_unseen.rs` covers each way a table's row may
resize with no row's `&mut` given to a call (#1849).
The historical suite totals and CI timings below have not been remeasured.

Changed on 2026-11-16: the copy matrix steps the whole catalog of
adapters, `filter_map` and the other step adapters too, and three more
routes (`by_ref`, `dyn`, a callee stepping it back): 53,360 cases, 447 s
whole on the mini in 10 parts. Added `test/adapter-catalog.test.ts`, which
holds the catalog to std's own list of adapters.

Added on 2026-11-16: the copy matrix, `ITERATOR_TABLE=copies`
(`test/copy-matrix.ts`, ADR 1090, amended): each compiled iterator form
copied by `.clone()`, a clone in generic code or a move through a generic
identity, then stepped by each ability beside its original, about 23,700
cases in both builds against native Rust. Its slice runs in each push's
`nets-iterators` job and `check:fast` where the iterators' code changes, some 25 s on
the mini in 10 parts. The nightly's `copies` job runs it whole, 205 s on the
mini in 10 parts.

Changed on 2026-11-16: `test/ci.test.ts`'s tests of which mutations a change
touches are `test/mutation-changes.test.ts`, split by concept near the cap.

Added on 2026-11-16: `test/ci-spend.test.ts`, the nightly's CI spend report
(`scripts/ci-spend.ts`, docs/dev-loop.md's "CI spend"): each waste, the
cap's minutes, push-to-verdict, the flags and the days kept in the comment
on #1989. `test/ci.test.ts` holds the gate rule: every job of a push's
check is gated or is on `alwaysRun` with its reason.

Added on 2026-11-16: `test/attribute.test.ts`, a failed check's
attribution (`scripts/attribute.ts`, ADR 1101, amended): the batch's
commits its failing tests run again at, at most six, and the first each
fails at. check.yml's `attribute-plan`, `attribute` and `attribute-report`
run only after a failed suite; every suite run writes its JUnit report now.
A check's suite takes one machine of a small selection (`plan`'s
`test_shards`), and the Vite example's build runs in `lint`, its own job
gone.

Added on 2026-11-16: `test/clusters.test.ts`, the issues' kind × area count
(`scripts/clusters.ts`, ADR 1085, amended): its cells, its threshold of 3,
each cluster's refactor read from docs/bug-kinds.md's table, and the
nightly's comment on #1681.

Changed on 2026-11-16: the test selector (`scripts/select.ts`, ADR 1101,
amended). `bun run push`'s check (`check:fast`) and CI's suite run only the
test files and corpus cases a change can affect: of a change to `src/`,
those whose functions the test map recorded it touching; of a script, a
helper or a doc, those that import or read it; of a test file or a case,
itself; everything where it can't say. The test map records every file of
the push suite now, not the plain suites', the metamorphic sample in parts,
after each check of `main` on CI (`test-map.yml`), and `bun run push`
fetches it. CI's `plan` writes the selection, `selection.json`, which the
test shards run (`bun run test --selection=`), the shards skipped where it
selects none. The nightly runs everything and audits the selection
(`scripts/selector-audit.ts`): a test it failed that no push selected is a
P1 `selector: …`. New: `test/select.test.ts` (the rules, the real suite's
imports and reads, check:fast's jobs, push's line, one OS's map serving the
other, the audit). The plain suites' list, `plainSuites`, is gone.

Changed on 2026-11-15: the declarations' TypeScript callers
(`test/declarations-conformance.test.ts`, 101 cases, some 22 s) run in each
push's `declarations` job, and `check:fast`, where the change adds or edits
a corpus case, not only the nightly's: a new case's `pub struct Sized` broke
the harness unseen. The harness names std's items by their paths.

Added on 2026-11-15: `test/deep-chain-next.test.ts`, the deep-chain
harness's second step, in the browser shard, some 5 s warm: a Next.js app
over the chain's crates, each a path dependency of its crate, built by
`rust-js build` and `next build`, prerendered, and in Chromium its dropdown
opened where geometry places it of the page's layout, flipped above a
trigger near the viewport's bottom, closed as an item is chosen, and its
window listener removed as it unmounts.

Added on 2026-11-15: `test/deep-chain.test.ts`, the deep-chain harness's
first step, one test of about 10 s on a seeded target. An app four crates
deep over the react crate (app, kit and forms, primitives, geometry; kit
and forms a diamond over primitives) is built by Cargo through rust-js;
what it computes is what native Rust prints, its page renders on the
server, and a TypeScript caller of the kit and forms crates type-checks
under strict tsc through each crate's declarations.

Added source-owner mutation coverage on 2026-11-15 (#1848): six real
workspace/load/control/cleanup tests and five self-mutants through the
existing runner. Feature-owner registrations are reviewed separately.
Added focused #1861 coverage on 2026-11-15: a seventh source-workspace case checks a partial worker-copy failure; three child cases retain in-budget exits under a blocked loop, observed-overrun warnings and external-signal provenance; a timing case checks that a passing check job still shows the warning with its name. Local whole-gate comparison on ef8807570: 6,915 existing test-name/status multisets unchanged, five new passing names, none removed; nine of nine harness mutants caught. Recorded lines: 6,469 pass/616 skip before, 6,474 pass/616 skip after (repeated names included). Local --all samples, not a nightly/CI recount.

Added #1867 coverage on 2026-11-15: a dedicated real-WebAPI aggregate-argument test covers objects and arrays, getter evaluation order and every thrown-value boundary, plus generated-source shape. Native owner counterexamples reject getters, computed keys, spreads and closure-written values. Two mutations remove the fix or its safety predicate. The initial object regression fails on main after ten runtime rows pass. The expanded focused case passes 34 assertions and the native safety unit passes; all 18 moved/new owner mutations are caught. The first complete ec7990f gate finishes in 429 seconds with only a newly adopted snapshot mismatch and an improved readability baseline; both are regenerated and reviewed before the final suite check.

Added on 2026-11-22 (#2305, ADR 0561 amended): `test/compiled-std.test.ts`
makes the catalogue of std's compiled bodies again, one compile, and
compares it with `std/compiled-std.json` byte for byte.

Added on 2026-11-22 (#2301, ADR 1182): `test/worker.test.ts` compiles 26
corpus cases (a sample, plus every `library-refused` one) each build alone
and then as the worker's jobs, program-first and library-first, comparing
exits, output and files byte for byte, in about 9 s; the nightly's
determinism job runs every case. `test/architecture-statics.test.ts`
classifies each of the compiler's 29 statics. The corpus compiles a case's
two builds as one job, each test process keeping one worker for its jobs
(step 2), so a corpus part pays the compiler's start once.

Added #1899 coverage on 2026-11-15: four optional-call cases prove the
unchecked Node receiver, rustc's unsafe requirement, the safe helper's strict
receiver and ordinary extraction's strict receiver. With the existing checked
downcast cases, the focused run passes eight tests and 35 assertions. All five
added or retargeted path-owner mutants are caught. The complete local
check:fast --all passes in 403 seconds on bc26493 plus this patch. This is a
local gate measurement; historical suite totals below are not a recount.

Added on 2026-11-15: `test/intl-arguments.test.ts` checks native Intl calls
in Bun and Chromium, derives accepted Rust callers from pinned TypeScript
unions across 26 sites, checks declarations and rejects nonmembers (#1830, #1834).

Added #1833 coverage on 2026-11-15: six focused LocalKey restoration cases
cover eager/generic callback proof, lazy/throwing and unrelated-callee controls,
result/exception identity and double-panic termination. All six pass with 52
assertions; eight new owner mutants are caught. A new native corpus program
agrees with Rust, and its dyn-fn metamorphic rewrite exposes a separate
pre-existing wrong answer, reported with a seven-line baseline reproduction.
That regression remains failing; no waiver or known-failure entry is added.
The first full gate also requires the new facts total (67) and the full new
module path in the architecture. Exact results are retained by the port.

Split on 2026-11-24 (#2465, B.1): the crate pairs' table, one test of each
`test/crates/pairs/<case>/`, is `test/crates-pairs-1.test.ts` to `-3`, each the
pairs a hash of the name puts in its part (`scripts/shard.ts`' `partOf`, a
fixed `cratePairParts`, as a file is the pool's unit), through
`test/crates-pairs.ts`; `test/crates.test.ts` keeps the tests that read what a
pair's builds write, and the triples (71). CI's 96 s, 69 s of it the pairs
(check 45344065161); on the M5 Pro, quiet, the compile cache off: 44.5 s
before; 15.5 s, 12.2 s (75), 10.0 s (60) and 11.3 s (71) side by side after,
277 tests as before.

Split on 2026-11-24 (#2465, A.3): `test/snapshots-serde.test.ts` holds the
snapshots of serde's examples (`wire`, `inbox`, `api`, `dynamic`, `wide`: 5
tests), moved from `test/snapshots.test.ts` (42), so `inbox`'s compile, the
snapshots' longest (45.5 s of CI's 92 s), starts with the pool; both run their
table through `test/snapshot-cases.ts`. On the M5 Pro, quiet, the compile
cache off: 17.5 s before; 7.8 s and 11.2 s side by side after.

Split on 2026-11-24 (#2465, B.4): `test/compat-surfaces.test.ts` holds what
rust-js's own wasm-bindgen, js-sys and web-sys serve of each crate that uses
them (inline JS imports, getrandom's backends, chrono's `wasmbind`,
`#[wasm_bindgen_test]`s, the patch of a build's features, `unwrap_throw`: 6
tests), moved from `test/compat.test.ts`, which keeps the graphs and locks (5
tests); what both share is `test/compat.ts`. On the M5 Pro, quiet: one file
of 11 tests, 63.6 s, before; the two side by side, 43.9 s and 30.1 s, after.

Split on 2026-11-22: `test/crates-refused.test.ts` holds the pairs of crates
under test/crates/refused/, a consumer refused for what a library can't give
it, moved from `test/crates.test.ts`, which neared its 1,350-line cap (#1334).

Added on 2026-11-21: `test/io-readers.test.ts` holds the receiver box a known
byte source's call is given to the runtime: each source's dictionary is called
with a box whose setter throws, and throws exactly where the compiler gives a
handle (#1329). One test, about 1 s.

As of 2026-10-30, `main` at `120772e2`; times are CI's, check run 41022723550.

| Suite | Size | What it proves | Time |
|---|---|---|---|
| `bun run test` | 3,851 tests in 175 files | Everything below except mutations, rustc, fuzz and the nightly's files | 11–13 min on 2 machines: each one's corpus parts, 8.5–9.6 min, then its browser files |
| ↳ the nightly's files (`nightly` in `scripts/shard.ts`, ADR 1030) | 24 files, ~136 tests: tidy's stages, the read cases, the object model's sample (#891), iterator forms, determinism (#866), the `&mut` matrix, the iterator matrix's sample, the declarations' generated TypeScript callers (ADR 1091; each `None` form a `.d.ts` admits, of 86 cases, #822, which a change to what a `None` is at the JS boundary runs per push too), fuzz's seeds, generated match trees (#771), the release's packaging, React's releases, the editor's check, the harness, build history, the tooling net's tree shaking; not the guide's limitations or the bindings' coverage ratchets, which a change can leave untrue unseen, so each push and plain `check:fast` run them (ADR 1030, amended) | What each says; a mutation that names one still runs it | ~1,000 s of CPU, run only by the nightly's suite, `--nightly` |
| ↳ corpus | 3,028 Rust programs (`test/corpus/*.rs`) | Each one's binary's JS runs as native Rust's binary does, status, stdout and stderr, and matches its snapshot (ADR 1198); built as a program, run where its JS differs, and as a library too, each push and the nightly every one, a library build that differs from the program's run again only where it's marked `library-differs` (ADR 0360, #1920): on CI about 1.5 min more of each test shard's corpus parts, 772 s against 679 s | about half the suite |
| ↳ the other 171 files | ~2,360 tests | Lowering rules, diagnostics, bindings (js, webapi, React, Node), hosts (Vite, Next, cargo), `rust-js test` by each runner (node, bun, Vitest, Jest; ADR 1076), React's ported modules against their snapshots, docs, architecture | the rest |
| `cargo test` | 57 unit tests | Tidy's passes and decisions | <1 min |
| Mutations | ~7,700 (`scripts/mutations/`) | Each rule has a test that fails without it | a check runs only the changed ones: ~10 min on up to 16 machines; built up to 32 to a compiler (ADR 1031) |
| rustc's suite | 3,770 run-pass tests, 1,492 of them out of scope | rustc's own programs run the same as JS | a check runs a sample in 1 min; all of them take ~2 h 45 min |
| Generated programs (`test/fuzz.test.ts`, ADR 0092) | A seed's program, of a swarm of the generator's 13 families of statement (#1085), `Pl`'s places among them, each run under Miri first in the nightly's batch and as JS in both builds | Programs no one wrote run as native Rust does | per push the first 12 seeds; 600 a night under Miri; 720 seeds in 12 parts 65 s on the M6 under Miri (2026-11-09) |
| Tidy's stages, preparation's rewrites and the facts they read (`test/tidy-stages.test.ts`) | 14 corpus cases in the nightly's suite; every case a night, each at 16 stage ends, without each of preparation's 51 rewrites that changed it (two to six a case), without each fact a reader found in it that only permits (`--without-known`, two to five a case of 37), without the recogniser of each std function it used (`--without-recognizer`, some fifteen a case of 2,113, most refused), without everything MIR's intervals prove (`--without-intervals`, of a case whose JS it changes), and at the least of tidy and preparation both; the night's 600 generated programs the same | Each of tidy's stages, each of preparation's rewrites left out, and each fact forgotten that only permits, leaves JS that runs as native Rust does; and what no recogniser takes of a std function is refused, or runs as native Rust does (ADRs 0659, 1087, 1127, 1128) | in the nightly: its suite runs the 11, every case on 6 machines and the programs on 6 more; per push, of a change to tidy or preparation, the 11 and six programs (**Readability passes** workflow), and of a change to the intervals, the pointers' code or a `&mut`'s, the intervals' 8 without their proofs (`TIDY_VARIANTS=intervals`, check.yml's `nets`, #2112 B); the 10 in 31–38 s (before #847 added an 11th) alone on the M1, each case's variants compiled at once (#808) |
| Read cases (`test/getter-reads.test.ts`) | 5 libraries of 45 calls, each given counting proxies of what JS made or gives | What JS made is read as often, and in the order, after every variant as at the least (ADR 1087); and, where a call says Rust's value and reads (`//@ gives:`, `//@ logs:`), as native Rust reads it (#1908) | 16 s on the Mac; in the nightly's suite, and the per-push sample |
| Iterator forms (`test/iterator-forms.test.ts`) | 102 corpus cases in the nightly's suite; every case built with std's iterator structs alone (`--std-iterators`, `ITERATOR_FORMS=corpus`) a night, and per push of the iterators' code (#766) | That build runs as native Rust does, but the cases `differs` lists, which only shrinks (ADR 0908): one, #1864, a `try_rfold` of an `Option` (2026-11-15) | in the nightly: its suite runs the 102, and every case on 2 machines, 304 s each (nightly 41941482272); a push that changes the iterators' code runs every case, check.yml's `iterator-forms`, in 4 parts on a machine of its own: 298 s on CI, 335 s the job with its build cached (run 41977111573), 183 s on the Mac at a load of 20–30 (2026-11-05) |
| React Compiler (`test/react-compiler.test.ts`, `scripts/react-compiler-sweep.ts`, ADR 1142) | Per push, ~60 modules: the guide's output fences, examples/' committed JS and test/react-compiler/'s cases (3, of consumers' findings and programs); nightly, the 237 modules the 33 React test files' programs write | babel-plugin-react-compiler, at the version Vite's React plugin's preset resolves, compiles every component and hook rust-js emits, but `test/react-compiler-known.txt`'s, each an open `react-compiler` issue or failing as the hand-written JS it names does; an entry nothing fails of fails too | under 2 s on the Mac per push; the sweep about 12 min in one part on the Mac, 2 parts nightly (2026-11-14) |
| Determinism (`test/determinism.test.ts`, ADR 1098) | 2 corpus cases compiled 8 times each in the nightly's suite, the two whose JS was once two forms (#866); every case compiled twice a night (`DETERMINISM=corpus`), as a program and as a library | The same input gives the same JS, source map and manifest, byte for byte (ADR 1003) | in the nightly: its suite runs the 2, 4 s; every case on 2 machines; all of it 133 s on the Mac in 10 parts (2026-11-07). Per push, `test/architecture-maps.test.ts` refuses std's maps, whose walk differs from one process to the next |
| The `&mut` matrix (`test/mut-ref-matrix.test.ts`) | 210 generated programs, 14 pointees × 15 holders, each through 17 operations (#855; `borrowed`, `borrowed_mut` and `given_back`, std's impls a `&mut` meets by its type, #1459), each built twice, plain and `--std-iterators`; 12 in the nightly's suite and in `check:fast` where a `&mut`'s form or an operation on one changes; all 210 a night | Each `&mut` form meets each operation as native Rust does, but the programs `wrong` lists by their P0s, none since #636's, #638's and #1514's fixes, which only shrinks (ADR 1086) | in the nightly: its suite runs the 12, and all 210 on 1 machine; all of it 10.5 s on the Mac in 12 parts (2026-11-08) |
| The `None` matrix (`test/none-matrix.test.ts`, #855) | 580 generated programs, 12 places × 5 payloads × 10 writers, each `Some` and `None` read 9 ways, each built twice: a local, a plain, nullable and optional field (nested and of a value by their payloads), a function's result and parameter, plain and nullable, a nullable thread-local, a `Vec`'s item, a `RefCell` and a `&mut`; 6 in the nightly's suite and in `check:fast` where a `None`'s form changes; all 580 a night | Each place's `NoneForm` meets each writer and reader as native Rust does, but what `refused` and `wrong` list, which only shrink: none since #957's and #958's fixes (ADR 1096) | in the nightly: its suite runs the 6, and all 580 on 1 machine; all of it 26 s on the Mac in 12 parts (2026-11-08) |
| The memory net (`test/memory-net.test.ts`, `test/memory-net.ts`, #1172) | A seed's program of unsafe Rust over a few integer arrays, `Vec`s and boxed slices, of a swarm of 10 families: raw pointer arithmetic, `offset_from`, `transmute` between plain types, a `union`'s other field, `MaybeUninit` slots, a `MaybeUninit` of each width, `u8` to `u128` and the floats, written through `*mut u8` and `MaybeUninit<u8>` slices a byte at a time and by `copy_nonoverlapping`, a slice of a cast pointer, a typed array's bytes, `ptr::copy` and `write_bytes`, reads and writes at any byte, and generic code given raw pointers; each run under Miri, natively on wasm32 and as JS in both builds | Each runs as native Rust does, or is refused, each refusal ranked by its reason; the slice's and the nightly's seeds but what `test/memory-net-known.txt` lists, a refusal by its reason and a wrong answer by the form of the operation that first prints otherwise. Its first run, 400 seeds: 40 wrong, #1253, #1254 and #1255 (P0); the refusals ranked, the commonest #1256 and #1257. With #1256 supported, seeds 1 to 400: none wrong, 350 of each build run as Rust does, 50 stopped by #1257. With #1257 supported, seeds 1 to 400 and 5000 to 5399: none wrong, none refused, all 800 of each build run as Rust does (2026-11-11). With its bytes written to a `MaybeUninit` (#1279's shape, which only the Crater had found), 356 of those 800 have one, all run as Rust does; with #1279's fix taken back, 119 of seeds 1 to 400 are wrong in both builds, in each of its 7 forms (2026-11-11) | 400 seeds from the day's a night (`memory`, one machine of 4 parts); nightly 42927113223's 4 parts 2–3 min each, at most 0.3 GB each (2026-11-11); the slice per push of the pointers' code in a job of its own, `memory-net`, and in `check:fast`: 7 s on the M6 (2026-11-10) |
| The Rustlantis net (`test/rustlantis.test.ts`, `test/rustlantis.ts`, ADR 1125) | A seed's program of custom MIR made by Rustlantis at its pinned commit with rust-js's wasm32 patch, small ones, three functions of eight blocks, about 150 lines, each dumping its locals; UB-free and deterministic by construction; natively on wasm32 and as JS in both builds | Each runs as native Rust does, or is refused, or what it does is an open issue's in `test/rustlantis-known.txt`, by issue: a refusal by its message, a wrong answer by its program, a throw by its error. Its experiment, 1,000 small seeds: 395 as Rust, 281 refused, 299 threw and 25 printed otherwise, which found #1380 and #1381 (P0); with #1382 refused, of seeds 1,000 to 1,199, none outside the list | a push, the list's own test; the nightly, 1,000 seeds of the day's in 4 parts (`rustlantis`); 1,000 seeds 37 s in 12 parts on the Mac, its generator built once in 10 s (2026-11-11) |
| The metamorphic net (`test/metamorphic.test.ts`, ADR 1104) | Each corpus case that runs on the host, 1,858, rewritten each of 42 ways by `scripts/metamorph` (`main` outlined; `let`s moved into a box, an `Option`, an `Rc<RefCell>`, a field, an array of one and a tuple of one; reference and owned parameters through an identity trait; a function made generic; closures and functions each made the other; items moved into a library crate; `for` loops and `for_each` each made the other; a `match` of two arms an `if let` or a `let else`; methods and free functions each made the other; a trait's defaults written out in each impl; structs made tuple structs; and equivalence modulo inputs, each block no run reaches made `unreachable!()` and code never run added, ADR 1108; the case made loud, each of its types printing its drops and clones and each closure call and loop step a line, and four more with a panic at the middle event of each kind, #1203; and contexts, #1204: each iterator given on as an `impl Iterator`, each closure called through a `Box<dyn Fn>`, and nine neighbours each turning on one crate-wide fact, never run; and each consumer and adapter called in generic code, given another instance, #1237; and, the nightly's alone, after each statement each `RefCell` a local reaches said, its borrow flag or the lengths behind its guard, `observe-borrows` and `observe-lens`, #2481), each built twice, with each rewrite's compiled and refused variants counted, failing where every one of 5 or more was refused | Each rewrite rustc compiles runs as native Rust runs it, but those `test/known/metamorphic/` lists under their issues, which only shrink | per push one case in 128, 16 s, and every rewrite of each case the change adds or edits, at most 24 by a hash of their names, unreduced (`NET_REDUCE=0`), each part's lines as they come (`metamorphic-cases`, and `check:fast`'s, #1165: two cases 9 s on the M6, 2026-11-10; the 101 a `since` of cancelled and red checks named, 2026-11-11, took 871 s in 4 parts on the Mac, 330–350 s a part of 34 cases and a reduction's 8.7 GB native variant, which killed CI's job at 3.5 min with no line, hence the cap, and a native run's 4 GiB `RLIMIT_DATA` on Linux); every case a night (`metamorphic`, 3 machines of 4 parts by `scripts/parts.ts`, since nightly 42927113223's one machine of 4 ran 141, 146, 149 and 148 min, at most 0.4, 0.5, 2.3 and 0.5 GB, and was stopped at its 150 (#1306); each part's lines as they come, its peak memory and what held it said as it ends, and a part the machine kills said `killed (memory?)` with what it was running, where nightly 42884008739's ended in a bare exit 143 at 38 min, #1306); all of it 8–11 min on the Mac in 10 parts (2026-11-08); after a change to the compiler 5 min in 12 parts, its runs and native programs kept, the machine's (#1047, 2026-11-09), of the first 11 rewrites; round 3's nine more alone 5 min 33 s in 10 parts (2026-11-09); the two EMI rewrites alone, every case, 200 s in 12 parts (2026-11-09); the five loud rewrites alone 6.5 min in 12 parts, and the eleven contexts 47 min in 11 parts, most of it reducing their 68 failures; `opaque` and `dyn-fn` alone, unreduced, 135 s in 12 parts, 300 s each at most, one after another in each part (2026-11-10, DEVELOPMENT.md's slow spots) |
| The neighbour shuffle (`test/neighbours.test.ts`, ADR 1103) | The corpus's cases that can share a crate, 1,652 of 1,846, hashed by a seed, by name, into 128 groups of some 13, each group one crate of a module a case; each built twice | Each case runs as native Rust does beside its neighbours, but the pairs `test/known/neighbours/` lists under their issues, 14 under 8 (#949–#956, 2026-11-08), each compiled alone on each run so the list only shrinks | per push, the listed pairs and seed 0's first 2 groups, 25 s on the Mac; every group of the day's seed a night (`neighbours`, one machine of 4 parts by `scripts/parts.ts`, which the runner's shutdown stopped every night of 2026-11-08 to 11; seed 12460200's 4 parts on the 16 GB Mac held 5.4, 9.7, 9.7 and 7.2 GB at their peaks, nearly all of it a reduction's native variants, one 8.6 GB, which macOS lets grow and Linux stops at 4 GiB each, before the reducer's variants were held to a share of the memory, #1306; nightly 42927113223's, of seed 12460800, 10, 34, 13 and 13 min, at most 0.4, 2.8, 0.3 and 0.6 GB, held); one seed's 120 groups 61 s on the Mac in 12 parts (2026-11-08) |
| serde / serde_json probes (`bun scripts/serde-probes.ts`, #1532 S0) | 358 one-statement programs of `test/serde-probes`' generators, 4 drop probes among them, and each review issue's repro, 11; each built natively with the pinned serde and with rust-js three ways: a program, a library, and split into a producer library and its consumer, each compiled on its own | Each program's outcome each way on Bun and Node, compared with its record in `test/serde-probes`: none that gave native Rust's answer stops (2026-11-13, as a program and a library: 320 same, 46 refused, 3 native-invalid, none different, a panic compared by its message; split 319 same, its one more refusal `type_name` of a type parameter; `std_29`, of `usize`'s width, against a wasm32 build) | by hand, ~45 s |
| The JSON benchmark (`bun scripts/serde-bench.ts`, #1542, #1532 S0) | #1542's document at 1, 10 and 96 MB and four shapes (deep, wide, long strings, maps), each written and read typed and as a `Value`, on Bun, Node and native serde_json built optimized, beside each engine's `JSON`; the bundle sizes of `size-programs.ts`' programs | Its tables, cold and warm, peak memory, every sample kept: a measurement of a quiet machine, not a pass or a fail | by hand; `--quick` ~4 min |
| The reducer (`test/reduce.test.ts`, ADR 1107, #1088) | 7 tests of `scripts/reduce.ts` and the rewriter's reductions: a program rust-js refuses reduced to what it refuses, a test's failures reduced at once, each within what's left of its timeout (#1365), one that runs as native Rust does left as it is, each reduction smaller, a variant past its share of the memory stopped and kept as no failure (#1306); with Miri, the nightly fuzz job's, a step that's undefined behaviour never kept, and a program that's undefined behaviour not reduced (#1104) | A net's failure is reported with its program reduced to one that fails the same way: the metamorphic net's and the neighbour shuffle's, each failure not on its known list (`test/reduced.ts`); its variants together hold at most the run's memory budget, `memoryBudget()`, a part of a net in `n` parts its `n`th, each variant its share and at most 300 s, it and what it starts stopped past either (#1306) | in the suite, 8 s on the Mac (2026-11-09) |
| The object model's net (`test/object-model.test.ts`, ADR 1105, #891) | Each corpus case that runs on the host, 1,918, its items moved into a library (metamorph's `library`), the program run with each value the library gives it read through a counting `Proxy`, built as it is and at its least, each struct the library gives one JS may make (`--js-made-all`) | Each program reads what its library gave as often as its least JS does, and both run as native Rust does, but what `test/object-model-known.txt` lists under its issues, 10 under 3, #1066's and #1070's decided and #1067's to decide (2026-11-09), which only shrinks | in `check:fast`, one case in 64 and each listed one, where its change is the object model's owners or the readability passes; every case a night (`object-model`, one machine of 4 parts; nightly 42927113223's 8–9 min each, at most 0.5, 0.3, 0.3 and 0.3 GB); all of it 75 s on the 12-core M6 in 12 parts, its compiles kept (2026-11-09) |
| Iterator matrix (`test/iterator-matrix.test.ts`, ADR 1090) | 191,263 generated cases (2026-11-21; 163,711 before a sized `Box` of the crate's iterator and generic code's `Box<I>` and `&mut Box<I>` of it among the sources, #2219, ADR 1177; 2026-11-15: 132,791 before the nested item kinds, an `Option<D>`'s, a `Vec<D>`'s, a `(D, D)`'s and a struct's that holds a `D`, and the `map made` adapter, #1826; 116,248 before a `Vec<String>`'s sources, #1681's step (a), and the `extend` consumer), a boxed `dyn Iterator`, a binding's JS iterator and a `Vec<String>`'s, by value and borrowed, among the sources (ADR 1136): 101,973 of each source, chain of none to two adapters, consumer and panic site; 3,999 of them, of chains of none or one adapter, with each closure capturing a `D` by move; 1,200 with the type first stepped from its back by `rfold`, `nth_back` or `rfind` beside a forward consumer (#731); each built twice, plain and `--std-iterators` | Each case's lines, what it drops and when, and how it panics, are native Rust's in each build, but those `test/known/iterator-matrix/` lists under their issues, 342 of 214,344 case-builds, 242 of them the variants' (2026-11-05), which only shrinks | every case a night: ~4 CPU-hours, on 6 machines of 4 parts, about a quarter of an hour each after the compiler's build, estimated from the Mac's run of all of it, 2,970 s in 4 parts at a load of 15–60 (2026-11-04); its slice, 64 fixed batches, in a check whose change is the iterators' code (check.yml's `iterators`, `scripts/iteratorSlice.ts`) and in `check:fast`: on CI 89 s in 4 parts after a 116 s release build, the job 5 min wall beside the suite (run 41969266534); on the Mac 119–145 s in 4 parts at a load of 14–20 (2026-11-05); `ITERATOR_MATRIX=#<issue>`, the batches of one issue's known cases; and in `check:fast`, the rows a change names, of both tables (`scripts/iteratorRows.ts`, #818): each row of an adapter a hunk names, or `pairs` of them past a budget, and `pairs`, 200 batches of each adapter with each consumer, of a change to what they share: #834's 10,353 and 4,347 rows in 111 s in 12 parts, #810's reintroduced in 309 s (2026-11-06) |
| Iterator rows a change names (`test/iterator-rows.test.ts`, #818) | 5 tests of `scripts/iteratorRows.ts`' naming: a hunk's adapters by its identifiers and its comments' quotes, inflected; prose and the language's `for` naming none; consumers where it names no adapter; generic code's, the shared protocol and what's taken away; each name's rows | What a change runs of the matrix is what it names | in the suite, well under a second |
| Iterator conformance table (`ITERATOR_TABLE=conformance` of `test/iterator-matrix.test.ts`, ADR 1092) | 52,557 generated cases (2026-11-21; 41,742 before the three boxed sources of the crate's iterator, #2219; 2026-11-15: 33,976 before #1826's nested item kinds and `map made`; 29,830 before the `strings` sources and the `exact capacity` and `extend` consumers), each source alone and through each adapter alone, a `flat_map` of arrays and a `flatten` of `Option`s among them (#1323), each step of the contract that takes it, its hint asked by a function it's given to too, and each panic site, closures that hold a destructor too, in 498 programs of 60; each built twice | Each step of each adapter and of std's compiled structs drops, unwinds and asks what native Rust's does, but the `conformance:` rows of `test/known/iterator-matrix/`, 278 of 53,732 case-builds under 3 issues (2026-11-11), which only shrink | every case a night, on 2 machines of 4 parts (`conformance`); all of it 6 min on the Mac in 6 parts at a load of 20–37 (2026-11-05); its slice, 43 fixed batches, per push of the iterators' code in a job of its own, `iterator-conformance`, and in `check:fast` (#731): 62 s on CI in 4 parts after the build (run 41969266534) |
| Iterator paths table (`ITERATOR_TABLE=paths` of `test/iterator-matrix.test.ts`, `test/iterator-paths.ts`, #2112) | 5,877 generated cases (2026-11-24), of std's steps catalog: each adapter A of `crates/rust-js-lower/src/recognition/std_steps.txt`, each step s it writes, the consumer that asks s, by each of 16 paths (`&mut I`, `&mut dyn I`, `Box<I>`, `Box<dyn I>`, a generic function's parameter, an `impl Iterator` return, a clone, a `cycle().take(7)`; given through a trait `impl Is<I>` as itself, boxed and lent; `Box<Box<A>>`, `&mut Box<A>`, generic code's `Box<J>`; a library's generic function given it as itself and boxed, the library compiled apart), over the crate's iterators that write every step and none (a slice's items for `copied` and `cloned`), at sites `none` and `consumer`, in 99 programs of 60, a library path's cases in programs of their own, each with its library; each built twice | Each path reaches each adapter's steps as native Rust's does, none listed (2026-11-24) | all of it per push of the iterators' code, a step of `nets-iterators` (`bun scripts/iteratorSlice.ts run paths 2`), and in `check:fast`: 72 s in 2 parts on the M6 (2026-11-24) |
| Iterator panic points (`ITERATOR_TABLE=points` of `test/iterator-matrix.test.ts`, #817) | 13,254 generated cases (2026-11-15; 10,693 before #1826's nested item kinds and `map made`; 8,794 before 2026-11-14's `strings` and `extend` rows), each of the conformance table's that panics nowhere, in 147 programs of 60, each run once a panic point: each closure call, each drop and clone of a `D`, each drop of an accumulator or total, each `next`, `next_back` and drop of the crate's iterator; each built twice | Each step of each adapter unwinds from every point it has as native Rust does, but the `points:` rows of `test/known/iterator-matrix/`, 123 of 15,388 case-builds under 4 issues (#1180, #1326, #1327, #1330, 2026-11-11), which only shrink | every case a night (`points`, one machine of 4 parts); all of it 48 s on the Mac in 10 parts (2026-11-06) |
| Panic points beyond iterators, the unwind table (`ITERATOR_TABLE=unwind` of `test/iterator-matrix.test.ts`, `test/unwind-points.ts`, #854) | 3,470 generated cases in 58 programs of 60: each operation of 16 families, values and their fields, closures that hold what drops, `Vec`, `VecDeque`, `BTreeMap` (of values and of keys), `BTreeSet`, `HashMap` and `BinaryHeap` operations that call the crate's code, `Option` and `Result` combinators of either variant, `?` failing and passing, and comparisons, of each of 11 shapes of value it applies to (a `D`, structs and enums with and without a `Drop` of their own, a tuple, `Box`, `Option`, `Vec`, `Rc` and a closure that holds one), each run once a panic point: each make, drop, clone and comparison of a `D`, each drop of a struct or enum with its own `Drop`, each call of a closure or a fallible step; each built twice | Each operation unwinds from every point it has, dropping what native Rust drops in its order, but the `unwind:` rows of `test/known/iterator-matrix/`, 1,174 of 6,940 case-builds under 15 issues (#897–#912; #905 fixed, 2026-11-07), which only shrink; 106 of its cases a build are refused | every case a night (`unwind`, one machine of 4 parts); all of it 27 s on the Mac in 10 parts (2026-11-07) |
| The formatting grid (`ITERATOR_TABLE=format` of `test/iterator-matrix.test.ts`, `test/format-grid.ts`, #941) | 9,146 generated cases in 153 programs of 60: each of 60 format specs of the eight formatting traits, of each of 214 values of 53 types whose type has the trait, written by `println!`, `format!`, `write!`, a `panic!`'s message, `format_args!` or, of the 1,368 whose split `display::sink_writes` follows, every value but a map, a set and a generic field, floats, std's composites and the crate's own writers framed on its sink among them, `write!` into a writer of the crate's own that records each call (#1652), in turn; each built twice | Every placeholder's options write what native Rust writes, but the `format:` rows of `test/known/iterator-matrix/`, none (2026-11-08) | every case a night (`format`, one machine of 4 parts); all of it 32 s on the M1 in 8 parts; its slice, 22 fixed batches, per push of the formatting code in a job of its own, `format-grid`, and in `check:fast`: 4 s (2026-11-08) |
| The numbers grid (`ITERATOR_TABLE=numbers` of `test/iterator-matrix.test.ts`, `test/number-grid.ts`, #940) | 84,935 generated cases in 425 programs of 200: 114 integer and float operations in each of their forms, `as` between every pair of the 14 number types, of a value and of a literal a `let` binds (#1380), `parse` and `from_str_radix`, of each type's boundary values, `isize` and `usize` as wasm32's; each built twice | Every number operation, cast and parse makes what native Rust's release build makes, panics included, but the `numbers:` rows of `test/known/iterator-matrix/`, none (2026-11-11) | every case a night (`numbers`, one machine of 4 parts); all of it 81 s on the M1 in 8 parts; its slice, 7 fixed batches, per push of the numbers' code in a job of its own, `number-grid`, and in `check:fast`: 3 s (2026-11-08) |
| Panic points across `.await`, the async table (`ITERATOR_TABLE=async` of `test/iterator-matrix.test.ts`, `test/async-points.ts`, #942) | 126 generated cases in 3 programs of 60: 18 families of async bodies of 7 shapes, each run once a panic point, each make and drop of a `D` and each await; natively by a `block_on`, as JS by `js::settle`; each built twice; of them, 21 cases rust-js refuses by design, a future held past what may be seen or dropped unawaited (#965) | Each async body drops what native Rust drops as a panic unwinds across `.await`, and a future held or dropped where nothing can tell runs as native Rust's, but the `async:` rows of `test/known/iterator-matrix/`, none (2026-11-08) | every case a night (`async`); all of it per push of how a future awaits and drops, in a job of its own, `async-points`, and in `check:fast`: 7 s (2026-11-08) |
| The consumer net (`ITERATOR_TABLE=consumers` of `test/iterator-matrix.test.ts`, `test/consumer-net.ts`; its coverage `test/consumer-net.test.ts`) | 15,260 generated cases (2026-11-25; 14,024 before #2572's item-drop and source-drop panic points and its library caller): 153 templates, each std method that takes an iterator by value as its receiver or an argument, read from std's own source (171 found: 127 covered, 17 lazy adapters the matrix's, 27 unstable or std's own), each `Iterator` consumer that takes `self`, `zip`, `chain`, `eq`, `cmp` and the comparisons, each `Extend`, `FromIterator`, `Sum` and `Product` impl, `Vec::splice`, the `Debug` builders' `entries`, `decode_utf16`; of 145 sources of 32 kinds of item, the crate's own iterator that prints each step, clone and drop, and it behind each std adapter an in-place form stands for that gives its items as they are (`map`, `filter`, `inspect`, `skip`, `take`, `step_by`, `chain`, `rev`, `flat_map`, `fuse`, `take_while`, `skip_while`; #2112's Change A step 3f, 7,138 of the cases), std's own (`vec`, an array's, `repeat_n`, `repeat(..).take(..)`, `once`, `repeat`, a `map` of a `vec`) and an empty one; each as it is, with a closure's 2nd call, its source's 2nd step or its argument's panicking; each called from the case's own code and from two generic callers, given its source as a type parameter's value or made by an `F: Fn() -> I`, whose `I` is given as it is (ADR 1090, amended); each built twice | Each consumer steps, clones and drops what it's given, and its items, as native Rust does, but the `consumers:` rows of `test/known/iterator-matrix/`, 116 of 13,772 case-builds under 6 issues (2026-11-10), which only shrink; 6,160 case-builds are refused. A new std method that takes an iterator fails its coverage test until a template covers it or `excluded` says why | every case a night (`consumers`, one machine of 4 parts); all of it per push of the iterators' or the drops' code (`consumerPaths`), in a job of its own, `consumer-net`, and in `check:fast`: 25 s on the 12-core Mac in 8 parts, its generic callers added (2026-11-10) |
| The std net (`ITERATOR_TABLE=std` of `test/iterator-matrix.test.ts`, `test/std-net.ts`; its coverage `test/std-net.test.ts`, #1171) | 183 generated cases in 3 programs: 136 templates, each stable method of std's `Vec`, `VecDeque`, `String`, `BinaryHeap`, `BTreeMap`, `BTreeSet`, `HashMap`, `HashSet`, slices, cells and `Rc` that changes what it's called on in place or calls what it's given, read from std's own source (242 found: 202 covered, 40 that touch no item, a capacity's or a pointer's); of items that print each clone and drop, whose `Ord` and `Eq` count their calls, and a hash table's key whose `hash` alone counts them, printed, in a table that never grows (#1362); each as it is and with its closure's, comparison's or hash's 2nd call panicking (its 1st, of a B-tree's search, whose count std leaves unsaid), what's left printed, sorted where std leaves its order unsaid; each built twice | Each method gives back, leaves, clones and drops what native Rust does, and panics with its message; none of 358 case-builds refused, where a refusal would be pinned in `refused` with what it says and its issue; a refusal not listed, one that says otherwise and a listed case that runs each fail it. Its first run found two, `insert` past the end and `String::retain` under a panic (#1267, #1268, fixed). A new std method fails its coverage test until a template covers it or `excluded` says why | every case a night (`std`, one machine of 4 parts); all of it per push of std's collections', slices', strings' or cells' code (`stdNetPaths`), in a job of its own, `std-net`, and in `check:fast`: 15 s on the 12-core Mac in 3 parts (2026-11-11) |
| The twin net (`ITERATOR_ORACLE=std` of `test/iterator-matrix.test.ts`, `test/twin-net.ts`; its list's test `test/twin-net.test.ts`, #1205, ADR 1122) | The tables' cases with no native run: per push each table's slice; a night all of the conformance, points, unwind, consumer and std tables and the matrix's `pairs` sample, 53,082 cases, each run in the plain build, the runtime's hand-written helpers, and in the build of std's own iterator bodies, the oracle | The runtime's helpers print what std's own bodies print, case by case (49,554 judged, the rest refused by std's build or ended in); 35 helpers pitted, `pitted`'s; each of the 231 both builds import is in `shared` with why std's build has no body of its own there; a run fails on one in neither, a whole one on a helper `pitted` names that no case pits now, coverage lost; a whole run's drift, a list out of date that hides no gap, fails nothing and is blessed, `BLESS=1 bun test/twin-net.ts helpers`, the nightly's patch `bless-twin-net`, `bun run ci:bless nightly` (ADR 1122, amended) | each push of the iterators' code (`twinPaths`), each table's slice, check.yml's `twin-net` and `check:fast`'s: 63 s on the Mac in 4 parts (2026-11-11); all of it a night (`twins`, `run twinsAll`), 56 s on the 12-core Mac in 12 parts, 9 min 15 s CI's job of it per push (run 42887915279), so a push runs the slices |

- **A push that changes the iterators' code** (`iteratorPaths` in `scripts/iteratorSlice.ts`) runs the corpus with std's iterator structs on a machine of its own beside the suite, the user's choice of 2026-11-05, and, as steps of the `nets-iterators` job, each a few minutes, the matrix's slice, the conformance table's, the paths table whole and the copy matrix's slice; the rows each push names, on machines of their own, `iterator-rows`, as many as `plan`'s pick of them needs and none of a push that names none (#2374); and of the `nets-programs` job the consumer net, as a change to the drops' code does, and the twin net, each table against std's own bodies (#1205); the three jobs of nets, `nets-iterators`, `nets-programs` and `nets`, hold every short net a change runs, each a step named for it (dev-loop change 3); the `wasm` job and rustc's tests run only of a change to what they build and run, and the mutations of a change that touches one; `test/ci.test.ts` holds each setting a test file reads to a part that runs per push or at night (`parts` in `scripts/shard.ts`); each is read through `partSetting` (`test/part-settings.ts`), and a run that sets one and runs no file that reads it fails as it ends (`test/setup.ts`, `test/part-settings.test.ts`, #1210): `METAMORPHIC=all bun test test/corpus.test.ts` ran no rewrite and passed, which hid a P0.
- **A check** (each push to `main`, which runs to its end; `bun run ci:check` of another branch) runs everything except the full rustc suite, fuzz and the nightly's files, in parallel: about **15 min**.
- **The nightly's test times** (`test-times`, scripts/testTimes.ts, `test/test-times.test.ts`): each job's, step's, test file's and test's time of the night, bun's JUnit reports and GitHub's job times, the top 25 in its summary and what grew slower than the last night's.
- **The nightly** runs the suite with its own files, every mutation, the 600 programs, every corpus case after each of tidy's stages, every case with std's iterator structs alone, every case compiled twice (#866), every case of the iterator matrix, the crate corpus, the small Crater (each corpus crate's own `#[test]`s natively and as JS, ADR 1109) and React's port: **~2–3.5 h**, a red night opening an issue, `Nightly red: <date>`.
- **The crate corpus** (`bun scripts/crate-corpus.ts --check`, the nightly's `crates` job and a job of `check:fast --all` since #857, which a refusal that stopped itertools showed only the nightly ran, ADR 1067; and since #1188 a push's own job, `crate-corpus`, and plain `check:fast`'s, of a change to what compiles it, `cratePaths` in `scripts/iteratorSlice.ts`: the compiler, its runtime, std's crate or the corpus's files, 95 s of a runner beside the suite, as 9353847cd refused itertools again unseen; and React's port, its own job `react-in-rust-port`, of a change to the compiler, its runtime, std's crate, the tooling or the bindings, `portPaths`, about 100 s of a runner, #1188) compiles each of `docs/crate-corpus.md`'s 100 crates, a push's check its first table's 23 and one in 8 of the 77 top crates by downloads, a new set each commit (`--sample`, #1174), the nightly all, each at the exact release its row names, with every crate it uses at the release its lock, `scripts/crate-corpus/<name>.lock`, names (`--locked`), and runs its probe beside native Rust, failing where a crate does worse than its row says; a row marked with an open issue, `runs (refused: #467)`, holds while the crate does just that. `--check` also holds the functions of crates' JS that `snapshotted` lists (216 lines of itertools' adaptors.js, diff.js and ziptuple.js) to `scripts/crate-corpus/snapshots/`, which `BLESS=1` writes: shapes no corpus case makes, which four of #1979's survivors moved alone (ADR 1067, amended). The nightly's `test/crate-snapshots.test.ts` runs it of each snapshotted crate, the test those mutations name, about 16 s of a mutant in its own target. All 100 take 124 s on the Mac from an empty `target/crate-corpus` and 94 s with their builds kept, each graph refused somewhere built again every run, where the first 23 take 30 s and 6 s (2026-11-10, #1174); all 22 took 35 s from empty half the cores' crates at once (2026-11-06; 4.4 min one at a time on 2026-11-03), Cargo's jobs at their default: its largest rust-js process is 436 MB, since regex-syntax's crash was fixed (#511).
- **React's port** (`bun scripts/react-in-rust.ts`, ADR 1067), react-in-rust at `ports/react-in-rust.pin`: `regen`, the nightly's `port` job, compiles its workspace to JS in one `rust-js build` (ADR 1068) and fails on a compile error or any file that differs from what the pin committed, 8 min cold on the Mac (475 s, 2026-11-02, against 36 min one crate at a time) and 2 s warm (40 min one crate at a time); on CI, 28 min one crate at a time with the release build cached (nightly 41417626908), not yet timed since; `check`, the **React port** workflow, of a push that changes `builtins/`, `webapi/`, `node/` or `react/` alone, `cargo check`s its workspace natively, no compiler built: 3 s on the Mac with the port fetched, 23 s on CI (run 41417596826).
- **The nightly, here**: `bun run nightly` runs each of its jobs on the Mac's cores, all of it in 77 minutes on the M3 Max (2026-10-30, `3dfa326a1`, 8,047 of 8,047 mutations caught in 65 of them), the suite with its own files, tidy's stages and the iterator forms of every case, the day's 600 programs, rustc's tests, the examples, the WASM compiler and every mutation; `bun run nightly <job>..` some of them.
- **On the Mac**, each command has a timeout of about twice its usual time, and runs on all the cores: the suite in parallel groups took about 3 minutes (2026-10-30), the corpus in 8 parts 90 s (AGENTS.md). `bun run check:fast --all`, each file and each of the corpus's 12 parts a job of its pool, takes about 3 min with no change on the 12-core M6, the cores 92% busy, and about 6 min with an iterator change's rows (2026-11-07, DEVELOPMENT.md); run again where the compiler's build is the same, a change to the tests or the docs alone, about 1 min 50 s, each compile kept by its inputs given again (`test/compiles.ts`, ADR 1099, #874). Plain `bun run check:fast`, the inner loop's, runs each plain suite's file as a job and, by the test map (`bun run test-map`, about 130 s), only the files and corpus cases that ran a function the change touches: 53–74 s of a one-statement change to tidy, drops, the bindings or an iterator's code, 121 s to the runtime's with its iterator rows (2026-11-07, ADR 1101, #877); `test/test-map.test.ts` holds what a change picks.

To recount: the test totals are the `Ran N tests` lines in a check's `test (k/2)` logs. The corpus is `ls test/corpus/*.rs | wc -l`, the mutations are `grep -rc 'name:' scripts/mutations`, and the rustc counts are the line counts of `test/rustc-*.txt`.

Added #1912 coverage on 2026-11-15: a separate three-case TypeScript-symbol suite covers computed-key model round trips, original/printed strict TypeScript consumers, literal string distinction and explicit unsupported-expression refusal (three pass, fourteen assertions). Seven new owner mutations cover the model, inventory formatting and browser consumer; all fourteen affected owner mutants are caught. The fix also passes independently with the published parser's original beta dependency. The complete `check:fast --all` gate on d2e47ad plus #1912/#1916 passes in 8 min 27 s: 321 other test files and nine browser files, all jobs green. This cold run rebuilt the compiler in 34 s, discarded compiles from another build and ran the std-iterator corpus whole without an ancestor test map; it is not an isolated performance comparison. Startup pruning duration is overstated by the synchronous-build observation bug tracked in #1917. Historical totals are not a recount.

Added #1916 coverage on 2026-11-15: two default-reader cases cover supported generic declaration shapes, exact round trips and original/printed strict callers, including literal and dependent defaults and negative constraints. Focused defaults pass two tests/11 assertions. Three new source mutants remove defaults, substitute constraints or lose constraints. This extends the existing TypeScript owner catalog; no second catalog is added. All ten newly added #1912/#1916 owner mutants are caught in the complete passing gate.

Added #1917 coverage on 2026-11-15: three startup tests use real child processes to prove that pruning is observed during the compiler build, both success/failure results survive, and startup waits for both children. Focused proof: three pass, 13 assertions. Four owner mutants cover blocking, a substituted duration, a missing build wait and a hidden pruning failure. Historical suite totals are not a recount.

#1917 complete validation: `check:fast --all` passes on 6963014 plus the startup fix in 5 min 23 s (322 other test files, nine browser files, all jobs green). The compiler build is warm at 439 ms with 58 MB kept compiles; this differs from the earlier cold reader gate and is not an isolated speed-up comparison. Both startup durations are now independently observed: pruning 7.6 s. Rebased onto published e0f07bc afterward; only incoming changes require follow-up checks under the moving-main rule.

After the #1917 rebase onto e0f07bc, incoming-scope checks pass: 17 startup/parser tests with 64 assertions, five declaration/forwarding tests with 25 assertions, 142 CI/architecture/catalog tests with 47,409 assertions, typechecking and all four startup mutants through the updated runner. The startup implementation/tests/catalog are byte-for-byte unchanged from the full-gate state; only the ledger insertion needed a merge, retaining both entries. Checks use the published compiler binary whose actual build.rs identity matches both source checkouts (99f613233edef5e7). No second whole-gate run is represented.

Added #1919/#1921 coverage on 2026-11-15: five cases in two new TypeScript suites pass 22 assertions in the focused run. Original/printed strict consumers and exact model round trips cover const inference, ordinary-inference controls, independent constraints/defaults, call/construct/method identity, overloads, nested object call signatures and optional/rest arguments. Eight added owner mutations target those boundaries. Historical suite totals are not a recount. The complete check:fast --all gate passes on 705057eb6 plus these fixes in about seven minutes: all 326 other test files and nine browser files. All eight added mutants are caught in the gate; all 13 new/affected mutants are caught in the separate focused run. An independent stable 7.0.2 probe passes the same five cases/22 assertions after only the reviewed stable API and fixture adaptations. The first bare focused invocation reached its 30-second bound in automatic compiler setup and yielded no behavior evidence; subsequent focused checks use the identity-verified published compiler override and actual pinned parser.

The #1919/#1921 gate rebuilt the current compiler in 55 seconds (build identity 9bc91a3415a2375b) and native tests took 56 seconds; compiles from another build were discarded (0 MB retained), and the std-iterator corpus ran whole without an ancestor map. The concurrent pruner is independently observed at 5.6 seconds. These are cold whole-gate costs, not an isolated speed-up comparison.

## The tooling net

The JS rust-js writes plays with the tools a frontend builds with (Tu's promise, 2026-11-24). Each tool is one suite over rust-js's own output:

| Tool | Suite | When |
|---|---|---|
| React Compiler | `test/react-compiler.test.ts`: every component and hook of the guide, `examples/` and `test/react-compiler/` compiles with no bailout but the known (ADR 1142); `scripts/react-compiler-sweep.ts`, the lowering tests' programs | Per push; the sweep nightly |
| Fast Refresh (HMR) | `test/vite.test.ts`: real Vite, React's plugin, React Compiler and Tailwind, a page in Chromium edited while mounted keeps its state | Per push |
| Minify | The corpus: each case's production build, bundled by Vite and minified by Oxc, runs as native Rust | Per push |
| Tree shaking, dead code | `test/tooling-tree-shaking.test.ts`: an app importing one export of a rust-js library bundles none of the others, and one importing it and using none of it bundles nothing of it, by Vite (Rolldown, minified) and Bun, an unused class too (#2495) | Nightly |

A new tool, or a new kind of item rust-js writes at a module's top, joins its row: nightly unless it's nearly free (Tu, 2026-11-24).
