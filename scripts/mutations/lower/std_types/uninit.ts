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
    name: "zeroed-bool-zero",
    breaks: "a zeroed `bool` is `0`, not `false`",
    file: "src/lower/std_types/uninit.rs",
    find: "            ty::Bool => Ok(Expr::bool(false)),",
    replace: "            ty::Bool => Ok(Expr::int(0)),",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "zeroed-struct-allowed",
    breaks: "a zeroed struct is `undefined`, a value no field is",
    file: "src/lower/std_types/uninit.rs",
    find: "            _ => Err(self.unsupported(span, &format!(\"a zeroed `{ty}`, which may be no value of it\"))),",
    replace: "            _ => Ok(Expr::undefined()),",
    tests: ["test/corpus.test.ts", "-t", "box_zeroed_struct"],
  },
  {
    name: "zeroed-slice-unfilled",
    breaks: "`new_zeroed_slice` leaves its slots `undefined`",
    file: "src/lower/std_types/uninit.rs",
    find: "                    return Ok(Some(Expr::call(Expr::member(made, \"fill\"), vec![zero.clone()])));",
    replace: "                    return Ok(Some(made));",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
];
