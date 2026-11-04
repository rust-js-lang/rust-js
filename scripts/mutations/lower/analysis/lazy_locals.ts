// Mutations of src/lower/analysis/lazy_locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const observable = ["test/corpus.test.ts", "-t", "^thread_local_lazy_where_observable.rs$"];
const byField = ["test/corpus.test.ts", "-t", "^thread_local_lazy_by_field.rs$"];

export const mutations: Mutation[] = [
  {
    name: "at-load-prints-unseen",
    breaks:
      "a thread-local whose initializer prints is made as its module loads, so `printed` comes before `main`'s `start`, where Rust prints it at its first use (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                        _ if printed => Some(Ordered::Prints),\n",
    replace: "                        _ if false && printed => Some(Ordered::Prints),\n",
    tests: observable,
  },
  {
    name: "at-load-panics-unseen",
    breaks:
      "a `pub` thread-local whose initializer may panic, `checked(2)`, is made as its module loads, where a panic would fail the load before what Rust does first (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                        _ if target.is_none() => Some(Ordered::Panics),\n",
    replace: "                        _ if false && target.is_none() => Some(Ordered::Panics),\n",
    tests: ["test/diagnostics.test.ts", "-t", "a pub thread-local whose initializer may panic"],
  },
  {
    name: "at-load-outside-writes-unseen",
    breaks:
      "a thread-local whose initializer reads `LEN`, which `main` sets before its first use, is made as its module loads of what `LEN` held then: `None` where Rust gives `Some(5)` (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "            for &place in &touched.writes {\n                add(&mut outside.writes, place, *id);\n",
    replace: "            for &place in touched.writes.iter().filter(|_| false) {\n                add(&mut outside.writes, place, *id);\n",
    tests: observable,
  },
  {
    name: "at-load-outside-reads-unseen",
    breaks:
      "a thread-local whose initializer bumps `COUNT`, which `main` reads before its first use, is made as its module loads, so `main` reads 1 where Rust reads 0 (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "        .find_map(|&place| Some((place.state, Outside::by(touches, &outside.touches, place)?)))\n",
    replace: "        .find_map(|&place| Some((place.state, Outside::by(touches, &outside.touches, place).filter(|_| false)?)))\n",
    tests: observable,
  },
  {
    name: "at-load-key-writes-unseen",
    breaks:
      "a thread-local's `set` isn't a write of it, so `LEN.set(5)` in `main` changes nothing an initializer reads, and `LAZY` is made of `LEN`'s 1 as its module loads (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                        if !read_only.contains(&at) && !tcx.is_mutable_static(def_id) {\n",
    replace: "                        if false && !read_only.contains(&at) && !tcx.is_mutable_static(def_id) {\n",
    tests: observable,
  },
  {
    name: "at-load-setter-read-only",
    breaks:
      "a thread-local's `set`, `replace` and `with_borrow_mut` are taken to only read it, so what `main` sets isn't seen and `LAZY` is made as its module loads (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                    && !local_key_sets(tcx, id)\n",
    replace: "",
    tests: observable,
  },
  {
    name: "at-load-static-mut-writes-unseen",
    breaks:
      "a `static mut` assigned in `main`, `LIMIT = 4`, isn't written outside the load, so `LIMITED`, which reads it, is made as its module loads (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                if let Some(def_id) = super::validation::static_of(thir, written)\n                    && def_id.is_local()\n",
    replace: "                if let Some(def_id) = super::validation::static_of(thir, written).filter(|_| false)\n                    && def_id.is_local()\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_reads_another_set.rs$"],
    snapshots: true,
  },
  {
    name: "at-load-nothing-changes",
    breaks:
      "no thread-local or `static` is state that may change, so `LAZY`, which reads `LEN`, is made as its module loads (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "        let mut changing = |id: DefId| *state.entry(id).or_insert_with(|| changes(tcx, id));\n",
    replace: "        let mut changing = |id: DefId| *state.entry(id).or_insert_with(|| changes(tcx, id) && false);\n",
    tests: observable,
  },
  {
    name: "at-load-struct-cells-unseen",
    breaks:
      "a thread-local of the crate's struct of a `Cell`, `SEEN`, isn't state that may change, so `SEEN_AT_FIRST` is made of its 0 as its module loads, where Rust reads the 2 `main` set (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                todo.extend(adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization()));\n",
    replace: "",
    tests: observable,
  },
  {
    name: "at-load-only-called-unloaded",
    breaks:
      "a function only the loading initializers call runs after the load too, so a `pub` thread-local of a cache only they touch, React's `ANIMATION_END`, is refused (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                loading.insert(f);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_exported_at_load.rs$"],
  },
  {
    name: "at-load-on-load-unloaded",
    breaks:
      "a `js::on_load!` runs after the load, so a `pub` thread-local reading the `Cell` it sets, React's `getVendorPrefixedEventName`'s `style`, is refused (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                .filter(|&id| crate::lower::bindings::is_on_load(tcx, id)),\n",
    replace: "                .filter(|&id| crate::lower::bindings::is_on_load(tcx, id) && false),\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "a pub thread-local is made as its module loads, which a JS importer reads"],
  },
  {
    name: "at-load-in-place-closures-unloaded",
    breaks:
      "a closure a loading function gives a thread-local's `with` runs after the load, so the cache it fills is written then, and a `pub` thread-local of that cache is refused (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "            if loading.contains(&at) {\n",
    replace: "            if false && loading.contains(&at) {\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_exported_at_load.rs$"],
  },
  {
    name: "at-load-kept-closures-in-place",
    breaks:
      "a closure std keeps, `Box::new(|| BUMPS.set(..))`, is taken to run as the loading function that makes it does, so `BUMPS_AT_FIRST` is made of 0 as its module loads, where Rust reads the 1 `main`'s call of it set (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "            if !kept {\n",
    replace: "            if true {\n",
    tests: observable,
  },
  {
    name: "at-load-once",
    breaks:
      "what a thread-local made where it's first used ran stays loading code, so `LOGS_AT_FIRST`, which reads what `LOGGED`'s initializer alone sets, is made of 0 as its module loads (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "        if shown.is_empty() {\n            break;\n        }\n",
    replace: "        if shown.is_empty() || !found.lazy.is_empty() {\n            break;\n        }\n",
    tests: observable,
  },
  {
    name: "at-load-pub-unrefused",
    breaks:
      "a `pub` thread-local whose initializer prints is made where it's first used, so JS's importers read the function `$local` makes, not its value (#554)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "            if tcx.visibility(key).is_public() {\n",
    replace: "            if false && tcx.visibility(key).is_public() {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a pub thread-local whose initializer prints"],
  },
  {
    name: "at-load-namespace-shows",
    breaks:
      "a namespace thread-local, whose first use is its module's load as only JS reads it, is asked whether its making then would show, so React's noop renderer's `pub static NOOP: Renderer = create(3)`, whose `create` counts what it makes, is refused (#554, ADR 1061)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "            .filter(|(_, key)| !crate::lower::classes::is_value_namespace(tcx, key.to_def_id()))\n",
    replace: "            .filter(|(_, key)| true || !crate::lower::classes::is_value_namespace(tcx, key.to_def_id()))\n",
    tests: ["test/namespace-values.test.ts", "-t", "fields are its module's exports"],
  },
  {
    name: "field-reads-whole",
    breaks:
      "what a `with`'s closure reads of a thread-local's struct isn't told apart by its fields, so `REGISTRY`, which reads `registry`, which nothing writes after the load, is made where it's first used for `main`'s write of `dispatcher`, `$local(..)` where React Flight's upstream destructures `TaintRegistryObjects` as its module loads (#616)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                        let Some(fields) = lent.get(&at) else {\n",
    replace: "                        let Some(fields) = lent.get(&at).filter(|_| false) else {\n",
    tests: byField,
    snapshots: true,
  },
  {
    name: "field-lent-whole-unseen",
    breaks:
      "a `with`'s closure that hands the struct it's lent whole to a function, `bump(whole)`, is taken to read none of its fields, so `WHOLE_REGISTRY` is made of its 2 as its module loads, where Rust reads the 12 `bump` set (#616)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "    (!other_use).then_some(fields)\n",
    replace: "    (other_use || true).then_some(fields)\n",
    tests: byField,
  },
  {
    name: "field-same-apart",
    breaks:
      "two places of one field are taken apart, so `main`'s write of `dispatcher` isn't seen and `DISPATCHER` is made of its 1 as its module loads, where Rust reads 5 (#616)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "        if a.state != b.state || f == g {\n",
    replace: "        if a.state != b.state || (f == g && false) {\n",
    tests: byField,
  },
  {
    name: "field-sets-unwritten",
    breaks:
      "a `Cell`'s `set` of a field isn't a write of it, so `main`'s `dispatcher.set(5)` isn't seen and `DISPATCHER` is made of its 1 as its module loads (#616)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                            if used.writes || through {\n",
    replace: "                            if through {\n",
    tests: byField,
  },
  {
    name: "field-pointees-apart",
    breaks:
      "two fields are always apart, so `Shared`'s `second`, an `Rc` of the cell `first` holds too, is written without `FIRST` seeing it, made of 1 as its module loads, where Rust reads 4 (#616)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "        (a.through && reaches(f, g, b.through)) || (b.through && reaches(g, f, a.through))\n",
    replace: "        false && ((a.through && reaches(f, g, b.through)) || (b.through && reaches(g, f, a.through)))\n",
    tests: byField,
  },
  {
    name: "field-given-out-unseen",
    breaks:
      "what a `Cell`'s `get` of a field gives out is taken to be read alone, so `Links`' `second.get()`, a reference to the cell `first` holds too, written through, isn't seen, and `LINKED` is made of 1 as its module loads, where Rust reads 8 (#616)",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                            let through = used.lends || (used.gives && points);\n",
    replace: "                            let through = used.lends || (used.gives && points && false);\n",
    tests: byField,
  },
];
