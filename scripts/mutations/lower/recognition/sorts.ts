// Mutations of src/lower/recognition/sorts.rs (ADRs 0342, 0471; #925, #936).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "stable-sort-items-of-sort-unasked",
    breaks: "a generic `sort()` of a type parameter's items asks its caller nothing, so what std's stable sort picks by them is missing (#925)",
    file: "src/lower/recognition/sorts.rs",
    find: '"sort" | "sort_by" | "sort_by_key"',
    replace: '"sort_by" | "sort_by_key"',
    tests: ["test/corpus.test.ts", "-t", "^generic_from_iter_in_place.rs$"],
  },
  {
    name: "ordering-eq-always-negated",
    breaks: "`o == Equal` is read as `o != Equal`, so a hand-written `then` by `==` is taken as maybe not total, and its sort is std's (#936)",
    file: "src/lower/recognition/sorts.rs",
    find: 'negated: name.as_str() == "ne",',
    replace: "negated: true,",
    tests: ["test/corpus.test.ts", "-t", "^sort_total_written_out.rs$"],
    snapshots: true,
  },
  {
    name: "tree-built-entries-set-pair",
    breaks: "a set's bulk build of a type parameter's items asks its caller the stable sort of a pair, not of its items (#926)",
    file: "src/lower/recognition/sorts.rs",
    find: "    Some(match map {\n        true => Ty::new_tup(tcx, &[key, of.types().nth(1)?]),\n",
    replace: "    Some(match true {\n        true => Ty::new_tup(tcx, &[key, of.types().nth(1).unwrap_or(key)]),\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_nodes_generic.rs$|^btree_own_ord.rs$"],
  },
];
