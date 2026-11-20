// Mutations of src/lower/mir/elaborated.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const flatten = ["test/drops-shadow.test.ts", "-t", "a match's enum moved in part and whole is dropped where rustc drops it"];

export const mutations: Mutation[] = [
  {
    name: "shadow-ladder-variants-unseen",
    breaks: "a `Drop` of rustc's ladder reached through a test of a discriminant is read as dropping every variant, so `flatten`'s moved `Text` bytes read as dropped by rustc where rust-js drops them never (#2112's change D)",
    file: "src/lower/mir/elaborated.rs",
    find: "        holds(&self.place, key)\n            && !(self.except.iter()).any(|(at, variant)| {\n",
    replace: "        holds(&self.place, key)\n            && !(self.except.iter()).any(|(at, variant)| false && {\n",
    tests: flatten,
  },
  {
    name: "shadow-flags-unread",
    breaks: "a `Drop` of rustc's behind a test of a drop flag is read as always run, so `flatten`'s rest of a moved enum, flagged on each side, reads as unalike (#2112's change D)",
    file: "src/lower/mir/elaborated.rs",
    find: "                flagged = true;\n",
    replace: "                flagged = false;\n",
    tests: flatten,
  },
  {
    name: "shadow-rest-at-the-top-only",
    breaks: "the rest of a value is compared at the value alone, not at each part with parts of its own, so `flatten`'s `(_15.0)`, which rustc drops as its own flag says, reads as unalike (#2112's change D)",
    file: "src/lower/mir/elaborated.rs",
    find: "        for &node in &inner {\n",
    replace: "        for &node in inner.iter().filter(|&&node| node == path) {\n",
    tests: flatten,
  },
  {
    name: "shadow-empty-variant-compared",
    breaks: "a variant none of whose fields JS drops, `Text(Vec<u8>)`, is compared, so `flatten`'s reads as rust-js asking a flag of what rustc's ladder never drops (#2112's change D)",
    file: "src/lower/mir/elaborated.rs",
    find: "            if self.variant_drops_nothing(body, move_data.move_paths[leaf].place) {\n",
    replace: "            if false && self.variant_drops_nothing(body, move_data.move_paths[leaf].place) {\n",
    tests: flatten,
  },
  {
    name: "shadow-derefer-copies-unread",
    breaks: "rustc's drop behind a pointer, `drop((*_t))` of `Derefer`'s `_t = deref_copy (_1.0)`, isn't read as the place it copies, so `*it = other`'s drop is found nowhere and unmatched (#2112's change D (b))",
    file: "src/lower/mir/elaborated.rs",
    find: "        if count == 1\n",
    replace: "        if count == 0\n",
    tests: ["test/drops-shadow.test.ts", "-t", "a drop behind a pointer is compared whole"],
  },
  {
    name: "shadow-behind-a-pointer-never",
    breaks: "a drop behind a pointer the writer drops is read as never dropped, so it disagrees with rustc's, where the writer and rustc agree (#2112's change D (b))",
    file: "src/lower/mir/elaborated.rs",
    find: "            LookupResult::Parent(_) => Some(if elaboration.drops(block, place) {\n",
    replace: "            LookupResult::Parent(_) => Some(if false && elaboration.drops(block, place) {\n",
    tests: ["test/drops-shadow.test.ts", "-t", "a drop behind a pointer is compared whole"],
  },
  {
    name: "shadow-alike-multiset-unread",
    breaks: "two drops written alike at one place, `?`'s, aren't compared as a multiset, so they stay unmatched (#2112's change D (b))",
    file: "src/lower/mir/elaborated.rs",
    find: "                Ok(_) if alike.len() > 1 && multiset() == Some(true) => out.push(line(\n",
    replace: "                Ok(_) if alike.len() > 1 && false => out.push(line(\n",
    tests: ["test/drops-shadow.test.ts", "-t", "two drops written alike are compared as a multiset"],
  },
  {
    name: "shadow-multiset-keeps-dead-drops",
    breaks: "a drop of what's not there is kept in rust-js's multiset, where rustc's elaboration leaves a dead drop out, so `?`'s two drops never match (#2112's change D (b))",
    file: "src/lower/mir/elaborated.rs",
    find: "                    if when != When::Never {\n",
    replace: "                    if true {\n",
    tests: ["test/drops-shadow.test.ts", "-t", "two drops written alike are compared as a multiset"],
  },
];
