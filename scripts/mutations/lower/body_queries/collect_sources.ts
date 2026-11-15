// Mutations of src/lower/body_queries/collect_sources.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "in-place-collect-source-unfound",
    breaks: "a `collect()` std makes in place of `v.into_iter()`'s chain finds no source `Vec`, so its capacity isn't known and `capacity()` refuses it (#1259)",
    file: "src/lower/body_queries/collect_sources.rs",
    find: "                {\n                    *collects.last_mut().expect(\"the collect's\") = Some(source);\n",
    replace: "                {\n                    let _ = source;\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collected_in_place_capacity.rs$"],
  },
  {
    name: "in-place-source-of-a-field-unfound",
    breaks: "an in-place collect of a field's `Vec`, `held.items.into_iter()`, finds no source, so its capacity isn't known and `capacity()` refuses it (#1259)",
    file: "src/lower/body_queries/collect_sources.rs",
    find: "            ExprKind::Field { lhs, name, .. } => {\n",
    replace: "            ExprKind::Field { lhs, name, .. } if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collected_in_place_capacity.rs$"],
  },
  {
    name: "in-place-source-of-a-box-unfound",
    breaks: "an in-place collect of a `Box`'s `Vec`, `(*boxed).into_iter()`, finds no source, so its capacity isn't known and `capacity()` refuses it (#1259)",
    file: "src/lower/body_queries/collect_sources.rs",
    find: "            ExprKind::Deref { arg } if thir[arg].ty.is_box() => at = strip(thir, arg),\n",
    replace: "            ExprKind::Deref { arg } if false && thir[arg].ty.is_box() => at = strip(thir, arg),\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collected_in_place_capacity.rs$"],
  },
  {
    name: "collects-at-one-span-of-sources-apart",
    breaks:
      "two `collect()`s a macro's repetition writes at one span, of sources `a` and `b`, each take `b`'s capacity, `1 3` where Rust's first is `1 10` (#1788)",
    file: "src/lower/body_queries/collect_sources.rs",
    find: "        if collects.iter().any(|source| *source != first) {\n",
    replace: "        if false {\n",
    tests: ["test/limitations.test.ts", "-t", "macro writes at one place"],
  },
  {
    name: "collect-source-not-through-calls",
    breaks:
      "a collect through a crate's function that returns its argument's iterator, `opaque(kept.into_iter())`, stops at the call, refused, where native Rust prints its source's capacity (#1934)",
    file: "src/lower/body_queries/collect_sources.rs",
    find: "                at = strip(thir, arg);\n                continue;\n",
    replace: "                break;\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_capacity_through_a_function.rs$"],
  },
];
