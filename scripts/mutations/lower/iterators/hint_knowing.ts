// Mutations of src/lower/iterators/hint_knowing.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "given-filter-map-ends-boxed",
    breaks: "a `filter_map` made knowing its back keeps each kept `Some` in its box, where it looks like `None`, and folds `{ value }`s (#473)",
    file: "src/lower/iterators/hint_knowing.rs",
    find: "            Some(unbox) => Expr::call_helper(\"$mapEnds\", vec![some, unbox.clone()]),\n",
    replace: "            Some(_) => some,\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_map_of_given_from_its_back.rs$"],
  },
  {
    name: "flattened-options-unknowing",
    breaks: "a flatten of `Option`s made in place, a filter of the `Some`s, isn't made knowing its back, so `rfold` asks its source's `back`, which JS's own filter hasn't: a `TypeError` (#1326)",
    file: "src/lower/iterators/hint_knowing.rs",
    find: "            .or_else(|| (self.recognition().flattened_options(ty)).map(|(source, _, mapped)| (source, mapped)))?;\n",
    replace: "            .or_else(|| (self.recognition().flattened_options(ty)).map(|(source, _, mapped)| (source, mapped)).filter(|_| false))?;\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_from_its_back.rs$"],
  },
  {
    name: "flattened-options-source-unread",
    breaks: "a flatten of `Option`s, `src.filter(present)`, is read as a `filter_map`'s, `src.map(f).filter(present)`, which it isn't, so it's refused from its back (#1326)",
    file: "src/lower/iterators/hint_knowing.rs",
    find: "            (false, _) => (filtered, None),\n",
    replace: "            (false, _) => return None,\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_from_its_back.rs$"],
  },
  {
    name: "somes-not-made-knowing",
    breaks:
      "`$somes` and `$someItems` of what generic code steps from its back aren't made of their source made knowing its back, so `filter_map(..).rev()` of what it's given steps nothing from its back (#2199, #2205)",
    file: "src/lower/iterators/hint_knowing.rs",
    find: "            js::ExprKind::Call(callee, args)\n                if let Some(helper) =\n                    ([\"$somes\", \"$someItems\"].into_iter()).find(|&h| callee.helper_name() == Some(h))\n                    && let [made] = args.as_slice() =>\n            {\n                (made, Err(helper))\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(filter_ends|filter_map_of_given_from_its_back).rs$"],
  },
  {
    name: "hint-knowing-flatten-ends",
    breaks: "a flatten made knowing its hint alone is `$flattenEnds`, which steps each inner by its back, so the crate's iterators, which have none, throw (#2209)",
    file: "src/lower/iterators/hint_knowing.rs",
    find: "                    return Expr::call_helper(\"$flatMap\", vec![source_value, each]);\n",
    replace: "                    return Expr::call_helper(\"$flattenEnds\", vec![source_value, each]);\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_made_knowing_its_hint_asks_no_back.rs$"],
  },
];
