// Mutations of src/lower/std_types/uninit.rs (ADR 0332).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "uninit-write-unwritten",
    breaks: "`slot.write(v)` writes nothing",
    file: "src/lower/std_types/uninit.rs",
    find: "                    out.push(StmtKind::Assign(place.clone(), value).at(self.js_span(span)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "uninit-write-object-boxed",
    breaks: "`write`'s `&mut` to an object is a box of it",
    file: "src/lower/std_types/uninit.rs",
    find: "                match self.is_boxable(held) {\n                    true => Expr::handle(place),",
    replace: "                match true {\n                    true => Expr::handle(place),",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "uninit-handle-kept",
    breaks: "`point.write(v)` writes through a getter and setter, not the place",
    file: "src/lower/std_types/uninit.rs",
    find: "                    js::ExprKind::Handle(place) => *place,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
    snapshots: true,
  },
  {
    name: "zeroed-slice-unfilled",
    breaks: "`new_zeroed_slice` leaves its slots `undefined`",
    file: "src/lower/plain.rs",
    find: "        return Expr::call(Expr::member(made, \"fill\"), vec![zero.clone()]);",
    replace: "        return made;",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
];
