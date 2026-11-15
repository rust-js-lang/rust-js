// Mutations of src/source_format/wrap.rs, each `jsx!` body laid out at
// rustfmt's width (#1901, ADR 1166, step B).
import type { Mutation } from "../../mutations";

const file = "src/source_format/wrap.rs";
const width = ["test/format.test.ts", "-t", "lays out a jsx! body at rustfmt's width"];
const long = ["test/format.test.ts", "-t", "a long child, a piece laid out again"];
const literals = ["test/format.test.ts", "-t", "preserves literal contents"];

export const mutations: Mutation[] = [
  {
    name: "wrap-verbatim-lines-indented",
    breaks: "a multi-line string's line in a piece is indented where it lands, `{\"first\\n second\"}`'s second line, which changes the program",
    file,
    find: "                    (true, _) => piece_line.clone(),\n",
    replace: "                    (true, _) => format!(\"{}{piece_line}\", \" \".repeat(indent)),\n",
    tests: width,
  },
  {
    name: "wrap-one-stem",
    breaks: "a body's placeholders are `Ω` whatever it holds, so a name or a comment holding `Ω` is read as one",
    file,
    find: "        .find(|stem| !written.contains(stem.as_str()))\n",
    replace: "        .find(|_| true)\n",
    tests: width,
  },
  {
    name: "wrap-no-second-round",
    breaks: "a piece still too long where it lands stays so, a line past rustfmt's width",
    file,
    find: "    if !again.is_empty() {\n",
    replace: "    if false {\n",
    tests: long,
  },
  {
    name: "wrap-long-child-hugged",
    breaks: "a child of several lines hugs its parent, `<ul className=\"list\">{items` and its chain after it, where Prettier puts it on a line of its own",
    file,
    find: "            _ if matches!(pieces[i].1, Form::Expression { child: true }) => width + 1,\n",
    replace: "",
    tests: long,
  },
  {
    name: "wrap-braces-left-broken",
    breaks: "an expression's braces oxc broke stay on lines of their own, `{` / the piece / `}`, where Rust's are hugged, `{items...}`",
    file,
    find: "                    if let (Some(_), Some(close), true) = (open, close, after.is_empty()) {\n",
    replace: "                    if let (Some(_), Some(close), true) = (open, close, false) {\n",
    tests: long,
  },
  {
    name: "wrap-piece-lines-absolute",
    breaks: "a piece's lines are given rustfmt at the file's indent, not its line's, so an opaque macro's body moves further each run",
    file,
    find: "        let keep = n == 0 || inside.get(n - 1).copied().unwrap_or(false);\n",
    replace: "        let keep = true;\n",
    tests: long,
  },
  {
    name: "wrap-bodies-unsorted",
    breaks: "bodies are replaced in the order the visitor meets them, a function's tail before its `match`'s, so one replaced first moves those after it",
    file,
    find: "    bodies.sort_by_key(|body| body.open);\n",
    replace: "",
    tests: long,
  },
  {
    name: "wrap-prop-literal-in-braces",
    breaks: "a prop's literal is laid out as `{Ω}`, whose braces oxc breaks of a long one, so `className=\"..\"` can't come back",
    file,
    find: "                true => out.push_str(&format!(\"\\\"{}\\\"\", marker(pieces.len() - 1))),\n",
    replace: "                true => out.push_str(&format!(\"{{{}}}\", marker(pieces.len() - 1))),\n",
    tests: width,
  },
];
