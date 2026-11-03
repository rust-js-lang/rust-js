// Mutations of src/lower/untagged/classes.rs (ADR 0093): how an untagged
// enum's class is told, a DOM node by its `nodeType`, or refused (ADR 1062,
// amended, #555).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "node-type-untold",
    breaks:
      "an untagged enum's DOM nodes are told by `instanceof Document`, `false` of another realm's document and a ReferenceError where the host has no `Document` (#555)",
    file: "src/lower/untagged/classes.rs",
    find: "            Some(Some((key, values))) => Kind::Member(key, Box::new(Kind::Constants(values))),\n",
    replace: "            Some(Some((key, values))) if false => Kind::Member(key, Box::new(Kind::Constants(values))),\n",
    tests: ["test/webapi-objects.test.ts", "-t", "told by their nodeType"],
  },
  {
    name: "untold-class-unrefused",
    breaks:
      "an untagged enum's classes no member tells, `Blob | Element`, `Date | RegExp`, are told by `instanceof`, `false` of another realm's (#555)",
    file: "src/lower/untagged/classes.rs",
    find: "                Kind::Class(class, Some(_)) => Some((v.name.to_string(), class.clone())),\n",
    replace: "                Kind::Class(class, Some(_)) if false => Some((v.name.to_string(), class.clone())),\n",
    tests: ["test/diagnostics.test.ts", "-t", "two classes told apart"],
  },
  {
    name: "untold-refused-in-arms-tests",
    breaks:
      "a `match` whose arms do the same JS, an `IntersectionObserver | ResizeObserver`'s `observer.observe(child)`, is refused where its tests are made, though none is left (#555)",
    file: "src/lower/untagged/classes.rs",
    find: "        if !self.telling_arms.get()\n",
    replace: "        if true\n",
    tests: ["test/lowering-switches.test.ts", "-t", "arms are alike in JS"],
  },
];
