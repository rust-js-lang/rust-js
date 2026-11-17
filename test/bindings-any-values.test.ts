// What a binding reads of any value, as JS reads it (ADRs 0310, 0364):
// react.dev's Console's reads, `concat_text`'s `Result` and `js::shown`, each
// compiled and run. Split from test/bindings.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildSerde } from "./serde-crate";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0310: what react.dev's Console reads of any value a page logs, as
// JS has it: a replace's callback of the match and its groups, `value + ""`,
// `parseInt` and `parseFloat` of any value and a number's `toString`,
// `JSON.stringify` of any value, which may throw or give `undefined`,
// `typeof`, and `Object.prototype.toString.call`.
test("any value is read as JS reads it, as react.dev's Console does", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("console-values");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsError, Unknown, json, number, object, reg_exp};

pub fn formatted(text: &str) -> String {
    let pattern = reg_exp::new("(%?)(%([jds]))", "g");
    reg_exp::replace(text, pattern, |matched: &str, escaped: Option<&str>, _ptn: Option<&str>, flag: Option<&str>| {
        if escaped.is_some_and(|e| !e.is_empty()) {
            matched.to_string()
        } else {
            format!("<{}>", flag.unwrap_or(""))
        }
    })
}

pub fn unescaped(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("%{2,2}", "g"), "%")
}

// Of any value, what may throw, of a symbol (ADR 0639).
pub fn text_of(value: Option<&Unknown>) -> Result<String, &'static JsError> {
    js::concat_text(value)
}

pub fn px(n: f64) -> String {
    js::concat_text(n) + "px"
}

pub fn counted(count: f64, noun: &str) -> String {
    js::concat_text(count) + " " + noun
}

pub fn anchor(id: f64) -> String {
    "#".to_string() + &js::concat_text(id)
}

pub fn styled(name: &str, value: f64) -> String {
    ";".to_owned() + name + ":" + &js::concat_text(value) + "px"
}

pub fn appended(mut s: String, n: f64) -> String {
    s += &js::concat_text(n);
    s.push_str(&js::concat_text(n));
    s + &js::concat_text(n)
}

pub fn int_of(value: Option<&Unknown>) -> String {
    number::to_string(js::parse_int(value, Some(10)), None)
}

pub fn float_of(value: &Unknown) -> String {
    number::to_string(js::parse_float(value), None)
}

pub fn json_of(value: &Unknown) -> Result<Option<String>, &'static JsError> {
    json::stringify_with(value, None, 2)
}

pub fn kind_of(value: Option<&Unknown>) -> String {
    js::type_of(value)
}

pub fn tag_of(value: Option<&Unknown>) -> Result<String, &'static JsError> {
    object::to_string(value)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('text.replace(pattern, (matched, escaped, ptn, flag) =>\n    escaped ? matched : `<${flag ?? ""}>`,\n  );');
  expect(js).toContain('return text.replace(/%{2,2}/g, "%");');
  expect(js).toContain('return $try(() => "" + value);');
  // Text joined to it already makes it text, as react.dev writes `right - left + 'px'`.
  expect(js).toContain('return n + "px";');
  expect(js).toContain('return count + " " + noun;');
  expect(js).toContain('return "#" + id;');
  expect(js).toContain("  s += n;\n  s += n;\n  return s + n;");
  expect(js).toContain("return parseInt(value, 10).toString();");
  expect(js).toContain("return parseFloat(value).toString();");
  expect(js).toContain("JSON.stringify(value, null, 2)");
  expect(js).toContain("return typeof value;");
  expect(js).toContain("Object.prototype.toString.call(value)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.formatted("a %s b %%d"), lib.unescaped("100%% sure")]).toEqual(["a <s> b %%d", "100% sure"]);
  expect([lib.textOf(undefined), lib.textOf(12), lib.textOf({})].map((r) => r._0)).toEqual(["undefined", "12", "[object Object]"]);
  expect(lib.textOf(Symbol("s")).TAG).toBe("Err");
  expect([lib.px(1.5), lib.px(-0), lib.counted(2, "videos"), lib.anchor(2), lib.anchor(1e21)]).toEqual(["1.5px", "0px", "2 videos", "#2", "#1e+21"]);
  // After text joined already, as React DOM's createDangerousStringForStyles
  // writes it (rd74).
  expect(js).toContain('return ";" + name + ":" + value + "px";');
  expect([lib.styled("width", 2), lib.styled("top", -0), lib.appended("a", 1)]).toEqual([";width:2px", ";top:0px", "a111"]);
  expect([lib.intOf("42px"), lib.intOf(undefined), lib.floatOf("1e21"), lib.floatOf("x")]).toEqual(["42", "NaN", "1e+21", "NaN"]);
  const cyclic: any = {};
  cyclic.self = cyclic;
  expect([lib.jsonOf({ a: 1 }), lib.jsonOf(() => 1)]).toEqual([{ TAG: "Ok", _0: '{\n  "a": 1\n}' }, { TAG: "Ok", _0: undefined }]);
  expect(lib.jsonOf(cyclic).TAG).toBe("Err");
  expect([lib.kindOf(undefined), lib.kindOf("s"), lib.kindOf(1n)]).toEqual(["undefined", "string", "bigint"]);
  expect([lib.tagOf([]), lib.tagOf(null)]).toEqual([{ TAG: "Ok", _0: "[object Array]" }, { TAG: "Ok", _0: "[object Null]" }]);
});

// ADR 0639: `value + ""` of a value of any shape may throw, of a symbol or
// of an object whose `valueOf` throws, so its result is a `Result`, which
// React's `willCoercionThrow` catches; of a number or text, it's `String`.
test("concat_text of any value is a Result, which a match catches", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("concat-text-throws");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn will_coercion_throw(value: Option<&Unknown>) -> bool {
    match js::concat_text(value) {
        Ok(_) => false,
        Err(_) => true,
    }
}

pub fn throws(value: &Unknown) -> bool {
    js::concat_text(value).is_err()
}

pub fn coercible(value: &Unknown) -> bool {
    js::concat_text(value).is_ok()
}

pub fn width(n: f64) -> String {
    js::concat_text(n) + "px"
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('function willCoercionThrow(value) {\n  try {\n    "" + value;\n    return false;\n  } catch {\n    return true;\n  }\n}');
  expect(js).toContain('function throws(value) {\n  try {\n    "" + value;\n    return false;\n  } catch {\n    return true;\n  }\n}');
  expect(js).toContain('function coercible(value) {\n  try {\n    "" + value;\n    return true;\n  } catch {\n    return false;\n  }\n}');
  // Of a number, nothing to catch.
  expect(js).toContain('return n + "px";');
  expect(js).not.toContain("$try");
  const lib = await import(join(dir, "lib.js"));
  const throwing = { valueOf() { throw new Error("no"); } };
  const values = [undefined, null, 1, "a", {}, 2n, Symbol("s"), throwing];
  expect(values.map(lib.willCoercionThrow)).toEqual([false, false, false, false, false, false, true, true]);
  expect([lib.throws(Symbol("s")), lib.throws(2n), lib.coercible(Symbol("s")), lib.coercible(2n)]).toEqual([true, false, false, true]);
  expect(lib.width(1.5)).toBe("1.5px");
});

// ADR 0312: any value shown as JS's template shows it, `${value}`, as
// react.dev's Console keys what it logged by `${msg}-${index}`.
test("js::shown is shown as a template shows any value", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("shown");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn key(msg: Option<&Unknown>, index: usize) -> String {
    format!("{}-{index}", js::shown(msg))
}

pub fn named(arg: Option<&Unknown>) -> String {
    format!(" {}", js::string(arg))
}

// A number as JS shows it, \`1e+21\`, as react.dev's copyright shows a year.
pub fn year(year: f64) -> String {
    format!("© {}", js::shown(year))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return `${$shown(msg)}-${index}`;");
  expect(js).toContain("return `© ${year}`;");
  expect((await import(join(dir, "lib.js"))).year(1e21)).toBe("© 1e+21");
  // `String(arg)` stays, which names a symbol a template throws on.
  expect(js).toContain("return ` ${String(arg)}`;");
  const { key, named } = await import(join(dir, "lib.js"));
  expect(named(Symbol("s"))).toBe(" Symbol(s)");
  expect([key(undefined, 0), key(null, 1), key({}, 2), key("a", 3)]).toEqual(["undefined-0", "null-1", "[object Object]-2", "a-3"]);
});

// ADR 0312: js::shown's text is a string, `${n}`, wherever it goes: alone,
// with another value alone, padded, by `to_string()`, in a map, in JSON.
// It was the value itself, a number in a `Vec<String>`, and `a + b` of two.
test("js::shown's text is text, alone or with values alone", async () => {
  const dir = fixture("shown-text");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
use std::collections::HashMap;
use std::fmt::{self, Display, Write};

pub fn length(n: f64) -> usize {
    format!("{}", js::shown(n)).len()
}

pub fn pushed(n: f64) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!("{}", js::shown(n)));
    out
}

pub fn both(a: f64, b: f64) -> String {
    format!("{}{}", js::shown(a), js::shown(b))
}

pub fn same(n: f64, s: String) -> bool {
    format!("{}", js::shown(n)) == s
}

pub fn keyed(n: f64) -> usize {
    let mut seen: HashMap<String, usize> = HashMap::new();
    seen.insert(format!("{}", js::shown(n)), 1);
    seen.get("5").copied().unwrap_or(0)
}

pub fn json(n: f64) -> String {
    serde_json::to_string(&format!("{}", js::shown(n))).unwrap()
}

pub fn padded(n: f64) -> String {
    format!("[{:>4}] [{:.1}]", js::shown(n), js::shown(n))
}

pub fn text(n: f64) -> usize {
    js::shown(n).to_string().len()
}

fn shown<T: Display>(value: T) -> String {
    format!("{value}")
}

pub fn generic(n: f64) -> usize {
    shown(js::shown(n)).len()
}

struct Hours(u32);

impl Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{}h", self.0)
    }
}

pub fn written(v: Option<&Unknown>, h: u32) -> String {
    let mut s = String::new();
    let _ = write!(s, "{}{}", js::shown(v), Hours(h));
    s
}

pub fn added(v: Option<&Unknown>) -> String {
    format!("{}!", js::concat_text(v).unwrap())
}

pub fn lines(n: f64) -> String {
    format!(
        "<b>
{}</b>",
        js::shown(n)
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...buildSerde(), "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return $byteLen(`${n}`);");
  expect(js).toContain("return `${a}${b}`;");
  expect(js).toContain("s += $shown(v);");
  // Its template spliced into one written across lines, which has no `\n`.
  expect(js).toContain("return `<b>\n${n}</b>`;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.length(5), lib.length(1e21), lib.length(-0)]).toEqual([1, 5, 1]);
  expect(lib.pushed(5)).toEqual(["5"]);
  expect(lib.both(1, 2)).toBe("12");
  expect([lib.same(5, "5"), lib.same(5, "6")]).toEqual([true, false]);
  expect(lib.keyed(5)).toBe(1);
  expect(lib.json(5)).toBe('"5"');
  expect(lib.padded(5)).toBe("[   5] [5]");
  expect([lib.text(1e21), lib.generic(1e21)]).toEqual([5, 5]);
  // An object as a template shows it, by its `toString`, not its `valueOf`.
  const odd = { toString: () => "t", valueOf: () => 1 };
  expect([lib.written(odd, 2), lib.written(odd, 30)]).toEqual(["t2h", "t"]);
  // `"" + v`, `concat_text`'s, by its `valueOf`, in a template too.
  expect(lib.added(odd)).toBe("1!");
});
