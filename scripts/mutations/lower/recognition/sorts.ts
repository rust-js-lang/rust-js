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
];
