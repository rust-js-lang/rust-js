// Mutations of src/lower/library/compiled_here.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-impl-not-compiled",
    breaks:
      "a library's impl its JS doesn't make, `Key`'s `Hash`, isn't compiled by its consumer, which then has no dictionary to give `hash_of` (ADR 0914, amended)",
    file: "src/lower/library/compiled_here.rs",
    find: "                DefKind::Impl { of_trait: true } => own(id),\n",
    replace: "                DefKind::Impl { of_trait: true } => false,\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "library-exported-impl-method-compiled",
    breaks:
      "the method of an impl whose dictionary a library exports is compiled by its consumer, not called through the dictionary (ADR 0914, amended)",
    file: "src/lower/library/compiled_here.rs",
    find: "                        Some(imp) => own(imp),\n",
    replace: "                        Some(imp) => self.has_impl(imp),\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "library-dictionary-entry-unseen",
    breaks:
      "a library's impl's method its JS reaches only through its exported dictionary is called as one it doesn't export, and refused (ADR 0914, amended)",
    file: "src/lower/library/compiled_here.rs",
    find: "        (!id.is_local() && !self.is_instance(id) && self.item(id).is_none() && self.item(imp).is_some()).then_some(imp)\n",
    replace: "        (false && !id.is_local()).then_some(imp)\n",
    tests: ["test/crates.test.ts", "-t", "may fail is one its consumer"],
  },
  {
    name: "compiled-not-an-instance",
    breaks: "a library's item the program has compiled itself already, an instance, isn't one it can call, so a library iterator's own `size_hint`, once called, is std's `(0, None)` to generic code (#1023)",
    file: "src/lower/library/compiled_here.rs",
    find: "        !id.is_local() && (self.item(id).is_some() || self.is_instance(id) || self.compiles(id))\n",
    replace: "        !id.is_local() && (self.item(id).is_some() || self.compiles(id))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_iterator_own_size_hint"],
  },
  {
    name: "writer-given-options-the-librarys",
    breaks:
      "a library's writer that reads its `Formatter`'s options, its library's JS taking none, is its import where the crate gives it a placeholder's options, `{j:>4}` and `{j:#x}` dropped (#1026)",
    file: "src/lower/library/compiled_here.rs",
    find: "            _ if self.writes.options && !library.writers_take_options => \"a placeholder's options, `{:>4}` or `{:#?}`\",\n",
    replace: "            _ if false && self.writes.options && !library.writers_take_options => \"a placeholder's options, `{:>4}` or `{:#?}`\",\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_writers_given_options"],
  },
  {
    name: "writer-given-debug-hex-the-librarys",
    breaks:
      "a library's writer that gives its `Formatter` on, its library asking no `{:x?}`, is its import where the crate asks `{:x?}` of it, which writes decimal (#1026)",
    file: "src/lower/library/compiled_here.rs",
    find: "            _ if self.writes.hex && !library.debug_hex => \"an integer's `{:?}` in hex, `{:x?}`\",\n",
    replace: "            _ if false && self.writes.hex && !library.debug_hex => \"an integer's `{:?}` in hex, `{:x?}`\",\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_writer_given_debug_hex"],
  },
  {
    name: "writer-reading-nothing-compiled",
    breaks:
      "every library writer is compiled where the crate's writers take options, one that only writes its text too, `hourDisplay_fmt` no longer the library's (#1026)",
    file: "src/lower/library/compiled_here.rs",
    find: "        let reads = |f: DefId| library.reads_formatter_options.contains(&key(tcx, f));\n",
    replace: "        let reads = |f: DefId| library.reads_formatter_options.contains(&key(tcx, f)) || f.krate == id.krate;\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "a library's generic function is its program's, alone"],
  },
  {
    name: "merging-function-the-librarys",
    breaks:
      "a library's function its manifest lists as merging what Rust keeps apart is its import where the crate compares an identity, its `Rc::ptr_eq` and `ptr::addr_eq` `true` (#1052)",
    file: "src/lower/library/compiled_here.rs",
    find: "            (self.writes_more(id).is_some() || self.merges_identities(id) || self.keeps_more(id))\n",
    replace: "            (self.writes_more(id).is_some() || self.keeps_more(id))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "merging-unlisted-of-its-callers",
    breaks:
      "a library's function that calls one that merges, `doubled` of `computed`, isn't listed, so the consumer calls its JS, whose boxes are one function (#1052)",
    file: "src/lower/library/compiled_here.rs",
    find: "            .filter(|(from, to)| merging.contains(to) && !merging.contains(from))\n",
    replace: "            .filter(|(from, to)| false && merging.contains(to) && !merging.contains(from))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "merging-impl-method-unresolved",
    breaks:
      "a call of a library trait's method that resolves to an impl the crate compiles itself, `Twice(5).pair()`, is taken to hand the crate the library's uncounted `Rc<dyn Fn>`s, refused (#1052)",
    file: "src/lower/library/compiled_here.rs",
    find: "            return self.merges_identities(instance.def_id());\n",
    replace: "            return false;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "merging-impls-of-generic-code-unasked",
    breaks:
      "a generic call of a library trait's method each library impl of which the crate compiles itself, `p.pair()` of a `P: Pair`, is taken to hand the crate the library's uncounted `Rc<dyn Fn>`s, refused (#1052)",
    file: "src/lower/library/compiled_here.rs",
    find: "        !impls.is_empty()\n",
    replace: "        false && !impls.is_empty()\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "merging-closures-unlisted",
    breaks:
      "a library's functions whose closures are the functions they pass on to, as it compares none, aren't listed, only its uncounted `Rc`s: its consumer calls its `computed`, `bumps` and `pings` (#1052)",
    file: "src/lower/library/compiled_here.rs",
    find: "            .filter(|(_, item)| item.dependencies.merges_identities)\n",
    replace: "            .filter(|_| false)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "dyn-method-resolved-to-the-trait",
    breaks:
      "a `dyn Trait`'s method called through a library-made `Box<dyn Trait>` resolves to the trait's own method, which merges nothing, so the `Rc<dyn Fn>`s it gives are a library's uncounted ones and ADR 0852 refuses the crate (#1055)",
    file: "src/lower/library/compiled_here.rs",
    find: "            && instance.def_id() != id\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_dyn_method_identities"],
  },
  {
    name: "capacity-kept-library-not-compiled-here",
    breaks:
      "a consumer that reads a `Vec`'s capacity calls the JS of a library that keeps none, so each `Vec` that library grows has its length for its capacity, silently: `squares(5)`'s 5 where std's is 8 (#1180's (5))",
    file: "src/lower/library/compiled_here.rs",
    find: "            .is_some_and(|library| !library.builtin && !library.keeps_capacity)\n",
    replace: "            .is_some_and(|library| !library.builtin && library.keeps_capacity)\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "keeps no capacity has its functions"],
  },
  {
    name: "capacity-kept-by-a-library-alone",
    breaks:
      "a consumer that reads a `Vec`'s capacity keeps none of what its libraries grow unless one of them keeps one too: `squares(5)`'s 5 where std's is 8 (#1180's (5))",
    file: "src/lower/library/compiled_here.rs",
    find: "!(self.asks_capacity || self.keeps_capacity())",
    replace: "!self.keeps_capacity()",
    tests: ["test/crates-compiled-here.test.ts", "-t", "keeps no capacity has its functions"],
  },
  {
    name: "capacity-not-a-reason-compiled-here",
    breaks:
      "a library function whose library keeps no capacity isn't compiled by a consumer that reads one: its `Vec`s' capacities are their lengths, silently (#1180's (5))",
    file: "src/lower/library/compiled_here.rs",
    find: "|| self.keeps_more(id))",
    replace: "|| false)",
    tests: ["test/crates-compiled-here.test.ts", "-t", "keeps no capacity has its functions"],
  },
];
