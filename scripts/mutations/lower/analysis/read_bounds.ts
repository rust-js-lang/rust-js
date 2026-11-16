// Mutations of src/lower/analysis/read_bounds.rs, which bounds of a
// generic function a body reads the dictionary of, so one nothing reads is
// given none (#1915, ADR 1165). A read missed is a dictionary not given
// where the body reads it, so the case's JS throws; one read where nothing
// does is a dictionary given that isn't, the case's snapshot.
import type { Mutation } from "../../../mutations";

const file = "src/lower/analysis/read_bounds.rs";
const tests = ["test/corpus.test.ts", "-t", "^bounds_given_a_dictionary_where_read.rs$"];
const crate = ["test/crates.test.ts", "-t", "a library's unread bound is given no dictionary by its consumer"];

export const mutations: Mutation[] = [
  {
    name: "read-bound-callee-unseen",
    breaks: "a function named or called isn't read for what it asks, so `eachShown`'s closure's `{:?}` is given no `TDebug`",
    file,
    find: "            self.named(callee, args);\n        }\n        // An associated constant",
    replace: "        }\n        // An associated constant",
    tests,
  },
  {
    name: "read-bound-assoc-const-unseen",
    breaks: "`T::NAME` isn't a read of `T: Named`, so `nameOf` is given no `TNamed`",
    file,
    find: "                Some(tr) if let Some(own) = unevaluated.args.types().next() => {\n                    self.asked(own, tr);\n",
    replace: "                Some(tr) if let Some(own) = unevaluated.args.types().next() => {\n",
    tests,
  },
  {
    name: "read-bound-dyn-unseen",
    breaks: "a `dyn` made of a `&T` doesn't read its bounds, so `asDyn`'s `dyn Speak` holds no `TSpeak`",
    file,
    find: "            let source = operand.ty(self.body, self.tcx);\n            self.all_of(source);\n",
    replace: "            let source = operand.ty(self.body, self.tcx);\n",
    tests,
  },
  {
    name: "read-bound-drop-unseen",
    breaks: "a drop of a type built of `T` doesn't read its bounds, so `guarded` drops its `Guard` with no `TSpeak` for its `Drop` impl",
    file,
    find: "            if !matches!(ty.kind(), ty::Param(_)) {\n                self.all_of(ty);\n",
    replace: "            if false && !matches!(ty.kind(), ty::Param(_)) {\n                self.all_of(ty);\n",
    tests,
  },
  {
    name: "read-bound-callee-bounds-unread",
    breaks: "a crate's callee's bound read only for being given, not as the fixpoint says, so `relay` gives `eachShown` no `TDebug`",
    file,
    find: "                    !self.candidates.contains(&bound) || self.read.contains(&bound)\n",
    replace: "                    !self.candidates.contains(&bound)\n",
    tests,
  },
  {
    name: "read-bound-supertrait-uncarried",
    breaks: "a bound's dictionary carries only its own trait, not one it extends, so `loud`'s `T: Loud` is unread for `speak()` and given none",
    file,
    find: "                rustc_middle::ty::elaborate::supertrait_def_ids(self.tcx, bound.2).any(|tr| tr == asked)\n",
    replace: "                bound.2 == asked\n",
    tests,
  },
  {
    name: "read-bound-impl-unselected",
    breaks: "a trait's method of `&T` reads every bound of `T`, not what the impl rustc selects asks, so `sameShown` is given an unread `TDebug`",
    file,
    find: "        if self.carried(own, tr) {\n            return None;\n        }\n",
    replace: "        if true || self.carried(own, tr) {\n            return None;\n        }\n",
    tests,
    snapshots: true,
  },
  {
    name: "read-bound-unread-taken-as-read",
    breaks: "no bound is unread, so `labeled` is given a `TDebug` it never reads",
    file,
    find: "    candidates.into_iter().filter(|bound| !read.contains(bound)).collect()\n",
    replace: "    candidates.into_iter().filter(|bound| false && !read.contains(bound)).collect()\n",
    tests,
    snapshots: true,
  },
  {
    name: "read-bound-library-contract-unread",
    breaks: "a library's function its contract says takes no dictionary is given one by its consumer, `labeled([1, 2], \"origin\", ..)`",
    file,
    find: "            || !id.is_local() && !foreign.reads(id, param.index, tr.def_id))\n",
    replace: "            || false && !id.is_local() && !foreign.reads(id, param.index, tr.def_id))\n",
    tests: crate,
  },
  {
    name: "read-bound-operator-reads",
    breaks: "a comparison by JS's own operator counts as a read of its bound, which is given a dictionary nothing reads (#1874)",
    file: "src/lower/analysis/read_bounds.rs",
    find: "        if self.compares_by_operator(callee, args) {\n            return;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^compared_by_operator.rs$"],
    snapshots: true,
  },
];
