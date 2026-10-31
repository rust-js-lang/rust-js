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
    replace:
      "                match true {\n                    true => Expr::handle(place),",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "uninit-handle-kept",
    breaks:
      "`point.write(v)` writes through a getter and setter, not the place",
    file: "src/lower/std_types/uninit.rs",
    find: "                    js::ExprKind::Handle(place, _) => *place,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
    snapshots: true,
  },
  {
    name: "uninit-array-undefined",
    breaks:
      "an array of `MaybeUninit`s not yet written, arrayvec's `ArrayVec::new`, is `undefined`, so writing an item through its pointer fails",
    file: "src/lower/std_types/uninit.rs",
    find: "                ty::Array(each, len) if self.recognition().uninit_of(each).is_some() => {\n",
    replace: "                ty::Array(each, len) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^zeroed_uninit_arrays.rs$"],
  },
  {
    name: "uninit-array-param-undefined",
    breaks:
      "`MaybeUninit::<A>::uninit()` of an array type parameter is nothing, which a pointer can't step into",
    file: "src/lower/std_types/uninit.rs",
    find: "                _ if std::iter::successors(Some(item), |&t| self.recognition().uninit_of(t))\n                    .any(|t| self.is_array_param(t)) =>\n",
    replace: "                _ if false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^uninit_array_param.rs$"],
  },
  {
    name: "uninit-of-uninit-array-param-undefined",
    breaks:
      "an uninit `MaybeUninit<A>` of an array type parameter is nothing, which a pointer can't step into",
    file: "src/lower/std_types/uninit.rs",
    find: "                _ if std::iter::successors(Some(item), |&t| self.recognition().uninit_of(t))\n                    .any(|t| self.is_array_param(t)) =>\n",
    replace: "                _ if self.is_array_param(item) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^uninit_array_param.rs$"],
  },
];
