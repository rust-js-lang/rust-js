// Mutations of src/lower/classes/constructors.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "class-constructor-of-an-rc-refused",
    breaks: "a class's constructor that gives an `Rc` of it, React's Flight server's `RequestInstance`, `fn new(..) -> Rc<Self>`, is refused, as a constructor that gives anything but `Self` was (rd554)",
    file: "src/lower/classes/constructors.rs",
    find: "    output == own || rc_pointee(tcx, output) == Some(own)\n",
    replace: "    output == own\n",
    tests: ["test/classes.test.ts", "-t", "a class's constructor that gives an Rc of it makes this"],
  },
  {
    name: "class-constructor-of-any-pointer-allowed",
    breaks: "a class's constructor that gives a `Box` of it, no `new`'s, compiles to a constructor function a Rust caller's `Box` reads as the instance (rd554)",
    file: "src/lower/classes/constructors.rs",
    find: "    output == own || rc_pointee(tcx, output) == Some(own)\n",
    replace: "    output == own || output.walk().any(|part| part.as_type() == Some(own))\n",
    tests: ["test/classes.test.ts", "-t", "what a class is refused says what to write"],
  },
  {
    name: "class-constructor-of-a-counted-rc-allowed",
    breaks: "a class's constructor that gives an `Rc` of it the crate counts, `{ value, strong, weak }`, sets the box's keys on `this`, `this.value = { count: 0 }`, whose `Rc::strong_count` reads `undefined` (rd554)",
    file: "src/lower/classes/constructors.rs",
    find: "            && counts(pointee)\n",
    replace: "            && counts(pointee) && false\n",
    tests: ["test/classes.test.ts", "-t", "a class's constructor that gives an Rc of it the crate counts is refused"],
  },
];
