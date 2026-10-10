// Mutations of src/lower/std_types/pin.rs (ADR 0329).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pin-mut-of-handle-unread",
    breaks: "`as_mut()` of a pinned `Box` of an object is the box of it, not the object",
    file: "src/lower/std_types/pin.rs",
    find: "            PinOp::Mut if handle && !self.is_boxable(target) => Expr::member(arg(), \"value\"),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
  {
    name: "pin-ref-of-box-unread",
    breaks: "`*pin` of a pinned `&mut` to a number is its box",
    file: "src/lower/std_types/pin.rs",
    find: "            PinOp::Ref => match self.boxed_target(pointer) {",
    replace: "            PinOp::Ref => match false {",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
  {
    name: "pin-set-assigns",
    breaks: "`set` of a pinned number replaces it as an object would be",
    file: "src/lower/std_types/pin.rs",
    find: "                match handle || self.is_boxable(target) {",
    replace: "                match false {",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
  {
    name: "pin-map-unapplied",
    breaks: "`map_unchecked` gives its pointer, not what `f` makes of it",
    file: "src/lower/std_types/pin.rs",
    find: "                apply(f, vec![pin])",
    replace: "                pin",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
];
