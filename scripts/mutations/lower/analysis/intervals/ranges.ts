// Mutations of src/lower/analysis/intervals/ranges.rs, which ranges a `for` loop's `next()` alone steps (ADR 1137, #1672 M3).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ranges-field-writes-unseen",
    breaks: "a range written a field at a time, `r.end = 3`, is taken to be one `next()` alone changes, so its loop's `v[i]` past the slice isn't checked",
    file: "src/lower/analysis/intervals/ranges.rs",
    find: "                PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call) => !projected,\n",
    replace: "                PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call) => true,\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
  {
    name: "ranges-reborrows-unfollowed",
    breaks: "a reborrow of a range's `&mut`, `&mut (*lent)`, as the session's MIR gives `next()`, isn't followed, so no for loop's value is known below its end",
    file: "src/lower/analysis/intervals/ranges.rs",
    find: "                        && matches!(of.projection.as_slice(), [mir::PlaceElem::Deref])\n",
    replace: "                        && false\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
];
