// Mutations of src/lower/bindings/given_before.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const refused = ["test/webapi-events.test.ts", "-t", "event methods keep target types"];

export const mutations: Mutation[] = [
  {
    name: "given-before-unread",
    breaks: "no trait is `#[rust_js::given_before]`, so `remove_event_listener(Click, &|e: &PointerEvent| ..)`, a new function, compiles and removes nothing (ADR 1021)",
    file: "src/lower/bindings/given_before.rs",
    find: "    tcx.get_attrs_by_path(def_id, &path).next().is_some()\n",
    replace: "    tcx.get_attrs_by_path(def_id, &path).next().is_some() && false\n",
    tests: refused,
  },
  {
    name: "given-before-borrow-unpeeled",
    breaks: "a closure borrowed where it's given, `&|e: &PointerEvent| ..`, which rustc makes `'static`, isn't one written in place, so removal of it compiles (ADR 1021)",
    file: "src/lower/bindings/given_before.rs",
    find: "            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = *arg,\n",
    replace: "            ExprKind::Deref { arg } => e = *arg,\n",
    tests: refused,
  },
  {
    name: "given-before-reborrow-unpeeled",
    breaks: "a closure reborrowed into a `dyn`, `EventListener(&|_| {})`, isn't one written in place, so removal of it compiles (ADR 1021)",
    file: "src/lower/bindings/given_before.rs",
    find: "            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = *arg,\n",
    replace: "            ExprKind::Borrow { arg, .. } => e = *arg,\n",
    tests: refused,
  },
  {
    name: "given-before-field-unpeeled",
    breaks: "a closure that's the field of what's made in place, `EventListener(&|_| {})`, isn't one written in place, so removal of it compiles (ADR 1021)",
    file: "src/lower/bindings/given_before.rs",
    find: "            ExprKind::Adt(adt) if adt.fields.len() == 1 => e = adt.fields[0].expr,\n",
    replace: "",
    tests: refused,
  },
];
