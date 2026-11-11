// Mutations of src/lower/body_queries/collect_sources.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "in-place-collect-source-unfound",
    breaks: "a `collect()` std makes in place of `v.into_iter()`'s chain finds no source `Vec`, so its capacity isn't known and `capacity()` refuses it (#1259)",
    file: "src/lower/body_queries/collect_sources.rs",
    find: "                    found.insert(fn_span, source);\n",
    replace: "                    let _ = (fn_span, source);\n",
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
];
