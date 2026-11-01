// Mutations of src/lower/mir/hints/clones.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clone-names-unfound",
    breaks:
      "no local is told as a `clone()`'s, so an `Rc`'s clone stays `const resolved = payload` (ADR 0846)",
    file: "src/lower/mir/hints/clones.rs",
    find: "self.tcx.is_lang_item(def_id, LangItem::CloneFn)",
    replace: "self.tcx.is_lang_item(def_id, LangItem::Clone)",
    tests: ["test/corpus.test.ts", "-t", "^rc_shared_by_closures.rs$"],
    snapshots: true,
  },
  {
    name: "kept-captures-of-borrows-too",
    breaks:
      "a capture a closure borrowed, which its maker gives another value later, is fixed in it, so a clone of it made in a call reads the new value (ADR 0879)",
    file: "src/lower/mir/hints/clones.rs",
    find: "        .filter(|(i, (_, place))| !place || fixed.as_ref().is_some_and(|fixed| fixed[*i]))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^branches_as_written.rs$"],
  },
  {
    name: "fixed-borrows-unread",
    breaks: "a clone a closure makes of a parameter it borrowed, `Rc::clone(self)`, keeps its own `const chunk = this;`, where upstream reads `this` (rd515, ADR 0846, amended)",
    file: "src/lower/mir/hints/clones.rs",
    find: "        .filter(|(i, (_, place))| !place || fixed.as_ref().is_some_and(|fixed| fixed[*i]))\n",
    replace: "        .filter(|(i, (_, place))| !place || fixed.as_ref().is_some_and(|fixed| fixed[*i] && false))\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_of_a_borrowed_parameter.rs$"],
    snapshots: true,
  },
  {
    name: "tuple-clones-unnamed",
    breaks: "a tuple of clones a pattern takes apart, `let (chunk, counted) = (Rc::clone(self), ..)`, isn't known for clones, so `const chunk = this;` stays where upstream reads `this` (rd515, ADR 0846, amended)",
    file: "src/lower/mir/hints/clones.rs",
    find: "        let locals: Vec<Local> = cloned.iter().copied().chain(taken).collect();\n",
    replace: "        let locals: Vec<Local> = cloned.iter().copied().chain(taken.filter(|_| false)).collect();\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_of_a_borrowed_parameter.rs$"],
    snapshots: true,
  },
];
