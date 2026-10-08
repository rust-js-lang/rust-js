// Writes src/intl.rs: JS's `Intl`, its formatters and their options (ADR
// 0283). TypeScript's `lib.es*.intl.d.ts` and ReScript's `Stdlib_Intl_*` are
// what it follows: each option a field of a struct whose `None` isn't given,
// each string union an enum, each formatter a type whose constructor and
// statics are a module of its name. The shape is written here once, as a
// spec, not read from TypeScript's declarations: it's the crate's own
// binding, kept by hand in this file, of a few hundred names alike.
//
//   bun builtins/intl.ts

const page = (path: string) => `https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Intl/${path}`;
const snake = (s: string) => s.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
/** A JS value's Rust variant: `"best fit"` `BestFit`, `"2-digit"` `TwoDigit`. */
const variant = (js: string) =>
  js.replace(/^2-/, "two-").split(/[^A-Za-z0-9]+/).map((w) => w[0].toUpperCase() + w.slice(1)).join("");

const out: string[] = [];
const line = (s = "") => out.push(s);
const close = () => {
  if (out.at(-1) === "") out.pop();
};

/** An enum of a string union's values, each its JS. */
const union = (name: string, doc: string, values: string[]) => {
  line(`/// ${doc}`);
  line(`#[derive(Clone, Copy, Debug, PartialEq, Eq)]`);
  line(`pub enum ${name} {`);
  for (const v of values) {
    line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(v)})]`);
    line(`    ${variant(v)},`);
  }
  line("}");
  line();
};

/** A field: its JS name, and its Rust type. */
type Field = [js: string, ty: string];
const field = ([js, ty]: Field) => {
  const rust = snake(js);
  const name = ["type"].includes(rust) ? `r#${rust}` : rust;
  if (rust !== js) line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(js)})]`);
  line(`    pub ${name}: ${ty},`);
};
/** What's given: each field an `Option`, `None` not given, as JS reads
 * an `undefined` option. A field given as required isn't. */
/** The options structs that borrow text, `<'a>`. */
const borrowing = new Set<string>();
const options = (name: string, doc: string, fields: Field[], required: string[] = []) => {
  const lifetime = fields.some(([, ty]) => ty.includes("'a"));
  if (lifetime) borrowing.add(name);
  line(`/// ${doc}`);
  line(`/// A JS object of these fields, a \`None\` one not given${required.length ? "" : ": `..Default::default()` the rest"}.`);
  if (!required.length) line("#[derive(Default)]");
  line(`pub struct ${name}${lifetime ? "<'a>" : ""} {`);
  for (const [js, ty] of fields) field([js, required.includes(js) ? ty : `Option<${ty}>`]);
  line("}");
  line();
};
/** What's read: a JS object's fields as it gives them. */
const record = (name: string, doc: string, fields: Field[]) => {
  line(`/// ${doc}`);
  line(`#[derive(Clone, Debug, PartialEq)]`);
  line(`pub struct ${name} {`);
  for (const f of fields) field(f);
  line("}");
  line();
};

/** A method: its doc, its JS, its Rust signature. */
const method = (doc: string, js: string, signature: string, indent = "    ") => {
  line(`${indent}/// ${doc}`);
  line(`${indent}#[cfg_attr(rust_js, rust_js::link_name = ${JSON.stringify(js)})]`);
  line(`${indent}pub fn ${signature} {`);
  line(`${indent}    unreachable!()`);
  line(`${indent}}`);
  line();
};

/** A formatter: its type, its methods, and its module's constructor and
 * `supportedLocalesOf`. */
const formatter = (name: string, doc: string, methods: [string, string, string][], ctor: { options: string; result?: string; doc?: string }) => {
  const js = `Intl.${name}`;
  const module = snake(name);
  line(`/// [\`${js}\`](${page(name)}): ${doc}`);
  line(`#[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(js)})]`);
  line(`pub struct ${name}(PhantomData<JsObject>);`);
  line();
  line(`impl ${name} {`);
  for (const [d, j, s] of methods) method(d, j, s);
  close();
  line("}");
  line();
  line(`pub mod ${module} {`);
  line("    use super::*;");
  line();
  method(
    ctor.doc ?? `[\`new ${js}(locales, options)\`](${page(`${name}/${name}`)}): one of the first of \`locales\` it has, or the user's of none, as \`options\` say.`,
    `new ${js}`,
    `new(locales: &[&str], options: &${ctor.options}${borrowing.has(ctor.options) ? "<'_>" : ""}) -> ${ctor.result ?? `&'static ${name}`}`,
  );
  method(
    `[\`${js}.supportedLocalesOf(locales)\`](${page(`${name}/supportedLocalesOf`)}): those of \`locales\` it has, without falling back to the user's.`,
    `${js}.supportedLocalesOf`,
    "supported_locales_of(locales: &[&str], options: &SupportedLocalesOptions) -> Vec<String>",
  );
  close();
  line("}");
  line();
};

const resolved = (name: string) => [`[\`${name.toLowerCase()}.resolvedOptions()\`](${page(`${name}/resolvedOptions`)}): the options it uses, as its locale resolved them.`, "resolvedOptions", `resolved_options(&self) -> Resolved${name}Options`] as [string, string, string];

line("//! JS's [`Intl`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Intl):");
line("//! text, numbers, dates, plurals and lists as a locale writes them (ADR 0283),");
line("//! as TypeScript's `lib.es*.intl.d.ts` and ReScript's `Stdlib_Intl` have it.");
line("//! A formatter is made of `locales`, `&[\"de-DE\"]`, the user's of none, and");
line("//! options, `&NumberFormatOptions { style: Some(NumberStyle::Currency), ..Default::default() }`,");
line("//! of which JS throws a `RangeError` of one it can't have.");
line("//! Generated by `builtins/intl.ts`; edit the shape there.");
line();
line("// A binding's parameters are its JS function's: its body never runs.");
line("#![allow(unused_variables)]");
line();
line("use core::cmp::Ordering;");
line("use core::marker::PhantomData;");
line();
line("use super::{Date, JsError, JsObject};");
line();

// ── What's shared ───────────────────────────────────────────────────────

union("LocaleMatcher", "How a locale is matched to `locales`: `localeMatcher`.", ["lookup", "best fit"]);
union("Style", "How wide a word is written: `\"long\"`, `\"short\"` or `\"narrow\"`.", ["long", "short", "narrow"]);
union("CaseFirst", "Which of upper and lower case is ordered first: `caseFirst`.", ["upper", "lower", "false"]);
union("HourCycle", "How hours are counted: `hourCycle`, `\"h23\"` from 0 to 23.", ["h11", "h12", "h23", "h24"]);
union("RangeSource", "Which of a range's ends a part is of: `source`.", ["startRange", "endRange", "shared"]);
options("SupportedLocalesOptions", "What `supported_locales_of` is given.", [["localeMatcher", "LocaleMatcher"]]);

// ── Intl ────────────────────────────────────────────────────────────────

union("SupportedValuesKey", "What `supported_values_of` lists.", ["calendar", "collation", "currency", "numberingSystem", "timeZone", "unit"]);
method(`[\`Intl.getCanonicalLocales(locales)\`](${page("getCanonicalLocales")}): \`locales\`, each as JS writes it, once, or the \`RangeError\` of one that isn't a locale.`, "Intl.getCanonicalLocales", "get_canonical_locales(locales: &[&str]) -> Result<Vec<String>, &'static JsError>", "");
method(`[\`Intl.supportedValuesOf(key)\`](${page("supportedValuesOf")}): the calendars, currencies, time zones or units it has.`, "Intl.supportedValuesOf", "supported_values_of(key: SupportedValuesKey) -> Vec<String>", "");

// ── Collator ────────────────────────────────────────────────────────────

union("CollatorUsage", "What a collator compares for: `usage`.", ["sort", "search"]);
union("Sensitivity", "Which differences between letters count: `sensitivity`.", ["base", "accent", "case", "variant"]);
options("CollatorOptions", "What [`collator::new`] is given.", [
  ["usage", "CollatorUsage"], ["localeMatcher", "LocaleMatcher"], ["numeric", "bool"], ["caseFirst", "CaseFirst"],
  ["sensitivity", "Sensitivity"], ["collation", "&'a str"], ["ignorePunctuation", "bool"],
]);
record("ResolvedCollatorOptions", "What a collator uses.", [
  ["locale", "String"], ["usage", "CollatorUsage"], ["sensitivity", "Sensitivity"], ["ignorePunctuation", "bool"],
  ["collation", "String"], ["caseFirst", "CaseFirst"], ["numeric", "bool"],
]);
formatter("Collator", "text compared as a locale orders it.", [
  [`[\`collator.compare(a, b)\`](${page("Collator/compare")}): how \`a\` is ordered to \`b\`, \`v.sort_by(|a, b| collator.compare(a, b))\`; JS's -1, 0 or 1 (ADR 0057).`, "compare", "compare(&self, a: &str, b: &str) -> Ordering"],
  resolved("Collator"),
], { options: "CollatorOptions" });

// ── NumberFormat ────────────────────────────────────────────────────────

union("NumberStyle", "What a number is written as: `style`.", ["decimal", "percent", "currency", "unit"]);
union("CurrencyDisplay", "How a currency is written: `currencyDisplay`, `\"symbol\"` `$`.", ["code", "symbol", "name", "narrowSymbol"]);
union("CurrencySign", "How a negative amount is written: `currencySign`, `\"accounting\"` `($1.00)`.", ["standard", "accounting"]);
union("Notation", "How a number is written: `notation`, `\"compact\"` `1.2K`.", ["standard", "scientific", "engineering", "compact"]);
union("CompactDisplay", "How a compact number's word is written: `compactDisplay`.", ["short", "long"]);
union("SignDisplay", "When a sign is written: `signDisplay`.", ["auto", "never", "always", "exceptZero", "negative"]);
union("GroupingMode", "When digits are grouped, `1,000`: `useGrouping`'s word.", ["min2", "auto", "always"]);
line("/// When digits are grouped: `useGrouping`, `false` never, or as its word says.");
line("#[cfg_attr(rust_js, rust_js::untagged)]");
line("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
line("pub enum UseGrouping {");
line("    Bool(bool),");
line("    #[cfg_attr(rust_js, rust_js::otherwise)]");
line("    Mode(GroupingMode),");
line("}");
line();
union("RoundingPriority", "Which of the fraction and significant digits wins: `roundingPriority`.", ["auto", "morePrecision", "lessPrecision"]);
union("RoundingMode", "How a number is rounded: `roundingMode`.", ["ceil", "floor", "expand", "trunc", "halfCeil", "halfFloor", "halfExpand", "halfTrunc", "halfEven"]);
union("TrailingZeroDisplay", "Whether a whole number's zero fraction is written: `trailingZeroDisplay`.", ["auto", "stripIfInteger"]);
union("NumberFormatPartType", "What a part of a written number is.", [
  "literal", "nan", "infinity", "percent", "integer", "group", "decimal", "fraction", "plusSign", "minusSign",
  "percentSign", "currency", "compact", "exponentInteger", "exponentMinusSign", "exponentSeparator", "unit", "unknown",
]);
const digits: Field[] = [["minimumIntegerDigits", "u32"], ["minimumFractionDigits", "u32"], ["maximumFractionDigits", "u32"], ["minimumSignificantDigits", "u32"], ["maximumSignificantDigits", "u32"]];
options("NumberFormatOptions", "What [`number_format::new`] is given: `currency` a code, `\"USD\"`, `unit` a unit, `\"kilometer\"`.", [
  ["localeMatcher", "LocaleMatcher"], ["style", "NumberStyle"], ["currency", "&'a str"], ["currencyDisplay", "CurrencyDisplay"],
  ["useGrouping", "UseGrouping"], ...digits, ["numberingSystem", "&'a str"], ["compactDisplay", "CompactDisplay"],
  ["notation", "Notation"], ["signDisplay", "SignDisplay"], ["unit", "&'a str"], ["unitDisplay", "Style"], ["currencySign", "CurrencySign"],
  ["roundingPriority", "RoundingPriority"], ["roundingIncrement", "u32"], ["roundingMode", "RoundingMode"], ["trailingZeroDisplay", "TrailingZeroDisplay"],
]);
record("ResolvedNumberFormatOptions", "What a number format uses.", [
  ["locale", "String"], ["numberingSystem", "String"], ["style", "NumberStyle"], ["currency", "Option<String>"],
  ["currencyDisplay", "Option<CurrencyDisplay>"], ["minimumIntegerDigits", "u32"], ["minimumFractionDigits", "Option<u32>"],
  ["maximumFractionDigits", "Option<u32>"], ["minimumSignificantDigits", "Option<u32>"], ["maximumSignificantDigits", "Option<u32>"],
  ["useGrouping", "UseGrouping"], ["compactDisplay", "Option<CompactDisplay>"], ["notation", "Notation"], ["signDisplay", "SignDisplay"],
  ["unit", "Option<String>"], ["unitDisplay", "Option<Style>"], ["currencySign", "Option<CurrencySign>"], ["roundingPriority", "RoundingPriority"],
  ["roundingMode", "RoundingMode"], ["roundingIncrement", "u32"], ["trailingZeroDisplay", "TrailingZeroDisplay"],
]);
record("NumberFormatPart", "A part of a written number: `formatToParts`'.", [["type", "NumberFormatPartType"], ["value", "String"]]);
record("NumberRangeFormatPart", "A part of a written range of numbers, and which end it's of.", [["type", "NumberFormatPartType"], ["value", "String"], ["source", "RangeSource"]]);
const nf = (m: string) => page(`NumberFormat/${m}`);
formatter("NumberFormat", "numbers written as a locale writes them.", [
  [`[\`format.format(n)\`](${nf("format")}): \`n\` written.`, "format", "format(&self, n: f64) -> String"],
  [`\`format.format(n)\` of a \`BigInt\`, an \`i64\` (ADR 0086), as ReScript's \`formatBigInt\`.`, "format", "format_big_int(&self, n: i64) -> String"],
  [`\`format.format(text)\` of a number's text, \`"1.10"\`, exact however long.`, "format", "format_string(&self, text: &str) -> String"],
  [`[\`format.formatToParts(n)\`](${nf("formatToParts")}): \`n\` written, part by part.`, "formatToParts", "format_to_parts(&self, n: f64) -> Vec<NumberFormatPart>"],
  [`[\`format.formatRange(start, end)\`](${nf("formatRange")}): the range written, \`$3–5\`.`, "formatRange", "format_range(&self, start: f64, end: f64) -> String"],
  [`[\`format.formatRangeToParts(start, end)\`](${nf("formatRangeToParts")}): the range written, part by part.`, "formatRangeToParts", "format_range_to_parts(&self, start: f64, end: f64) -> Vec<NumberRangeFormatPart>"],
  resolved("NumberFormat"),
], { options: "NumberFormatOptions" });

// ── DateTimeFormat ──────────────────────────────────────────────────────

union("NumericStyle", "How a number of a date is written: `\"numeric\"` `7`, `\"2-digit\"` `07`.", ["numeric", "2-digit"]);
union("MonthStyle", "How a month is written: `month`, `\"long\"` `July`.", ["numeric", "2-digit", "long", "short", "narrow"]);
union("TimeZoneNameStyle", "How a time zone is written: `timeZoneName`.", ["short", "long", "shortOffset", "longOffset", "shortGeneric", "longGeneric"]);
union("FormatMatcher", "How the parts asked for are matched to a locale's: `formatMatcher`.", ["basic", "best fit"]);
union("DateTimeStyle", "How much of a date or time is written: `dateStyle`, `timeStyle`.", ["full", "long", "medium", "short"]);
union("DateTimeFormatPartType", "What a part of a written date is.", [
  "day", "dayPeriod", "era", "hour", "literal", "minute", "month", "second", "timeZoneName", "weekday", "year", "fractionalSecond",
]);
const dateFields: Field[] = [
  ["weekday", "Style"], ["era", "Style"], ["year", "NumericStyle"], ["month", "MonthStyle"], ["day", "NumericStyle"],
  ["hour", "NumericStyle"], ["minute", "NumericStyle"], ["second", "NumericStyle"], ["timeZoneName", "TimeZoneNameStyle"],
];
options("DateTimeFormatOptions", "What [`date_time_format::new`] and a date's `to_locale_string_with` are given: `time_zone` an IANA name, `\"Asia/Tokyo\"`.", [
  ["localeMatcher", "LocaleMatcher"], ...dateFields, ["formatMatcher", "FormatMatcher"], ["hour12", "bool"], ["timeZone", "&'a str"],
  ["calendar", "&'a str"], ["dayPeriod", "Style"], ["numberingSystem", "&'a str"], ["dateStyle", "DateTimeStyle"],
  ["timeStyle", "DateTimeStyle"], ["hourCycle", "HourCycle"], ["fractionalSecondDigits", "u32"],
]);
record("ResolvedDateTimeFormatOptions", "What a date format uses.", [
  ["locale", "String"], ["calendar", "String"], ["numberingSystem", "String"], ["timeZone", "String"], ["hour12", "Option<bool>"],
  ...dateFields.map(([js, ty]): Field => [js, `Option<${ty}>`]), ["formatMatcher", "Option<FormatMatcher>"], ["dateStyle", "Option<DateTimeStyle>"],
  ["timeStyle", "Option<DateTimeStyle>"], ["hourCycle", "Option<HourCycle>"], ["dayPeriod", "Option<Style>"], ["fractionalSecondDigits", "Option<u32>"],
]);
record("DateTimeFormatPart", "A part of a written date: `formatToParts`'.", [["type", "DateTimeFormatPartType"], ["value", "String"]]);
record("DateTimeRangeFormatPart", "A part of a written range of dates, and which end it's of.", [["type", "DateTimeFormatPartType"], ["value", "String"], ["source", "RangeSource"]]);
const dtf = (m: string) => page(`DateTimeFormat/${m}`);
formatter("DateTimeFormat", "dates written as a locale writes them.", [
  [`[\`format.format(date)\`](${dtf("format")}): \`date\` written.`, "format", "format(&self, date: &Date) -> String"],
  [`[\`format.formatToParts(date)\`](${dtf("formatToParts")}): \`date\` written, part by part.`, "formatToParts", "format_to_parts(&self, date: &Date) -> Vec<DateTimeFormatPart>"],
  [`[\`format.formatRange(start, end)\`](${dtf("formatRange")}): the range written, \`Jan 10 – 20, 2007\`.`, "formatRange", "format_range(&self, start: &Date, end: &Date) -> String"],
  [`[\`format.formatRangeToParts(start, end)\`](${dtf("formatRangeToParts")}): the range written, part by part.`, "formatRangeToParts", "format_range_to_parts(&self, start: &Date, end: &Date) -> Vec<DateTimeRangeFormatPart>"],
  resolved("DateTimeFormat"),
], { options: "DateTimeFormatOptions" });

// ── PluralRules ─────────────────────────────────────────────────────────

union("PluralCategory", "Which of a locale's plural forms a number takes: `\"one\"`, `\"other\"`.", ["zero", "one", "two", "few", "many", "other"]);
union("PluralRuleType", "Of counting, `\"cardinal\"` `1 file`, or ordering, `\"ordinal\"` `1st`.", ["cardinal", "ordinal"]);
options("PluralRulesOptions", "What [`plural_rules::new`] is given.", [["localeMatcher", "LocaleMatcher"], ["type", "PluralRuleType"], ...digits]);
record("ResolvedPluralRulesOptions", "What plural rules use.", [
  ["locale", "String"], ["pluralCategories", "Vec<PluralCategory>"], ["type", "PluralRuleType"], ["minimumIntegerDigits", "u32"],
  ["minimumFractionDigits", "u32"], ["maximumFractionDigits", "u32"], ["minimumSignificantDigits", "Option<u32>"], ["maximumSignificantDigits", "Option<u32>"],
]);
formatter("PluralRules", "which plural form a number takes in a locale.", [
  [`[\`rules.select(n)\`](${page("PluralRules/select")}): the form \`n\` takes.`, "select", "select(&self, n: f64) -> PluralCategory"],
  resolved("PluralRules"),
], { options: "PluralRulesOptions" });

// ── RelativeTimeFormat ──────────────────────────────────────────────────

union("RelativeTimeUnit", "A unit of time: `\"day\"`. JS takes its plural too, `\"days\"`, the same.", ["year", "quarter", "month", "week", "day", "hour", "minute", "second"]);
union("RelativeTimeNumeric", "Whether a number is always written, `\"1 day ago\"`, or a word where there is one, `\"auto\"` `\"yesterday\"`.", ["always", "auto"]);
options("RelativeTimeFormatOptions", "What [`relative_time_format::new`] is given.", [["localeMatcher", "LocaleMatcher"], ["numeric", "RelativeTimeNumeric"], ["style", "Style"]]);
record("ResolvedRelativeTimeFormatOptions", "What a relative time format uses.", [["locale", "String"], ["style", "Style"], ["numeric", "RelativeTimeNumeric"], ["numberingSystem", "String"]]);
record("RelativeTimeFormatPart", "A part of a written time: its number's parts are of its `unit`, which a literal has none of.", [
  ["type", "NumberFormatPartType"], ["value", "String"], ["unit", "Option<RelativeTimeUnit>"],
]);
const rtf = (m: string) => page(`RelativeTimeFormat/${m}`);
formatter("RelativeTimeFormat", "a time from now written as a locale writes it, `in 3 days`.", [
  [`[\`format.format(value, unit)\`](${rtf("format")}): \`value\` \`unit\`s from now written, ago if negative.`, "format", "format(&self, value: f64, unit: RelativeTimeUnit) -> String"],
  [`[\`format.formatToParts(value, unit)\`](${rtf("formatToParts")}): written, part by part.`, "formatToParts", "format_to_parts(&self, value: f64, unit: RelativeTimeUnit) -> Vec<RelativeTimeFormatPart>"],
  resolved("RelativeTimeFormat"),
], { options: "RelativeTimeFormatOptions" });

// ── ListFormat ──────────────────────────────────────────────────────────

union("ListFormatType", "What a list says: `\"conjunction\"` `a, b, and c`, `\"disjunction\"` `a, b, or c`, `\"unit\"` `a, b, c`.", ["conjunction", "disjunction", "unit"]);
union("ListFormatPartType", "What a part of a written list is: an item, or what's between.", ["element", "literal"]);
options("ListFormatOptions", "What [`list_format::new`] is given.", [["localeMatcher", "LocaleMatcher"], ["type", "ListFormatType"], ["style", "Style"]]);
record("ResolvedListFormatOptions", "What a list format uses.", [["locale", "String"], ["style", "Style"], ["type", "ListFormatType"]]);
record("ListFormatPart", "A part of a written list.", [["type", "ListFormatPartType"], ["value", "String"]]);
formatter("ListFormat", "lists written as a locale writes them.", [
  [`[\`format.format(list)\`](${page("ListFormat/format")}): \`list\` written.`, "format", "format(&self, list: &[&str]) -> String"],
  [`[\`format.formatToParts(list)\`](${page("ListFormat/formatToParts")}): \`list\` written, part by part.`, "formatToParts", "format_to_parts(&self, list: &[&str]) -> Vec<ListFormatPart>"],
  resolved("ListFormat"),
], { options: "ListFormatOptions" });

// ── DisplayNames ────────────────────────────────────────────────────────

union("DisplayNamesType", "What's named: a language, a region, a currency..", ["language", "region", "script", "calendar", "dateTimeField", "currency"]);
union("DisplayNamesFallback", "What's given of a code without a name: the code, or none.", ["code", "none"]);
union("LanguageDisplay", "How a language is named: `\"dialect\"` `British English`, `\"standard\"` `English (United Kingdom)`.", ["dialect", "standard"]);
options("DisplayNamesOptions", "What [`display_names::new`] is given; its `type` is required, as JS's is.", [
  ["localeMatcher", "LocaleMatcher"], ["style", "Style"], ["type", "DisplayNamesType"], ["languageDisplay", "LanguageDisplay"], ["fallback", "DisplayNamesFallback"],
], ["type"]);
record("ResolvedDisplayNamesOptions", "What display names use.", [
  ["locale", "String"], ["style", "Style"], ["type", "DisplayNamesType"], ["fallback", "DisplayNamesFallback"], ["languageDisplay", "Option<LanguageDisplay>"],
]);
formatter("DisplayNames", "languages, regions, currencies and scripts named as a locale names them.", [
  [`[\`names.of(code)\`](${page("DisplayNames/of")}): the name of \`code\`, \`"fr"\`, \`"US"\`; none of one without a name, where \`fallback\` is \`"none"\`.`, "of", "of(&self, code: &str) -> Option<String>"],
  resolved("DisplayNames"),
], { options: "DisplayNamesOptions" });

// ── Locale ──────────────────────────────────────────────────────────────

options("LocaleOptions", "What [`locale::new`] is given: each a part of the locale it makes, over its tag's.", [
  ["calendar", "&'a str"], ["caseFirst", "CaseFirst"], ["collation", "&'a str"], ["hourCycle", "HourCycle"], ["language", "&'a str"],
  ["numberingSystem", "&'a str"], ["numeric", "bool"], ["region", "&'a str"], ["script", "&'a str"],
]);
line(`/// [\`Intl.Locale\`](${page("Locale")}): a locale and its parts, \`en-Latn-US\`.`);
line(`#[cfg_attr(rust_js, rust_js::name = "Intl.Locale")]`);
line("pub struct Locale(PhantomData<JsObject>);");
line();
line("impl Locale {");
const lp = (m: string) => page(`Locale/${m}`);
for (const [js, ty, d] of [
  ["baseName", "String", "its language, script and region, `en-Latn-US`"], ["language", "String", "its language, `en`"],
  ["script", "Option<String>", "its script, `Latn`"], ["region", "Option<String>", "its region, `US`"],
  ["calendar", "Option<String>", "its calendar"], ["caseFirst", "Option<CaseFirst>", "which case it orders first"],
  ["collation", "Option<String>", "how it orders text"], ["hourCycle", "Option<HourCycle>", "how it counts hours"],
  ["numberingSystem", "Option<String>", "its digits"], ["numeric", "bool", "whether it orders numbers in text by their value"],
]) method(`[\`locale.${js}\`](${lp(js)}): ${d}.`, `get ${js}`, `${snake(js)}(&self) -> ${ty}`);
method(`[\`locale.maximize()\`](${lp("maximize")}): with the script and region most likely of its language, \`en\` \`en-Latn-US\`.`, "maximize", "maximize(&self) -> &'static Locale");
method(`[\`locale.minimize()\`](${lp("minimize")}): without what \`maximize\` would add.`, "minimize", "minimize(&self) -> &'static Locale");
method(`[\`locale.toString()\`](${lp("toString")}): its tag.`, "toString", "to_string(&self) -> String");
close();
line("}");
line();
line("pub mod locale {");
line("    use super::*;");
line();
method(`[\`new Intl.Locale(tag, options)\`](${lp("Locale")}): the locale of \`tag\`, \`"en-US"\`, as \`options\` change it, or the \`RangeError\` of a tag that isn't one.`, "new Intl.Locale", "new(tag: &str, options: &LocaleOptions<'_>) -> Result<&'static Locale, &'static JsError>");
close();
line("}");
line();

// ── Segmenter ───────────────────────────────────────────────────────────

union("Granularity", "What text is split into: `granularity`, `\"grapheme\"` what a reader sees as a letter.", ["grapheme", "word", "sentence"]);
options("SegmenterOptions", "What [`segmenter::new`] is given.", [["localeMatcher", "LocaleMatcher"], ["granularity", "Granularity"]]);
record("ResolvedSegmenterOptions", "What a segmenter uses.", [["locale", "String"], ["granularity", "Granularity"]]);
record("SegmentData", "A segment of text: what it is, where it starts in `input`, and whether it's a word.", [
  ["segment", "String"], ["index", "u32"], ["input", "String"], ["isWordLike", "Option<bool>"],
]);
line(`/// [What \`segment\` gives](${page("Segmenter/segment/Segments")}): the segments of a text.`);
line("pub struct Segments(PhantomData<JsObject>);");
line();
line("impl Segments {");
method(`[\`segments.containing(index)\`](${page("Segmenter/segment/Segments/containing")}): the segment \`index\` is in, none past its end.`, "containing", "containing(&self, index: u32) -> Option<SegmentData>");
close();
line("}");
line();
line("pub mod segments {");
line("    use super::*;");
line();
method("`Iterator.from(segments)`: each segment, in order, a JS iterator (ADR 0140): `segments::iter(words.segment(text))`.", "Iterator.from", "iter(segments: &Segments) -> Box<dyn Iterator<Item = SegmentData>>");
close();
line("}");
line();
formatter("Segmenter", "text split into what a locale sees as its letters, words or sentences.", [
  [`[\`segmenter.segment(input)\`](${page("Segmenter/segment")}): \`input\` split.`, "segment", "segment(&self, input: &str) -> &'static Segments"],
  resolved("Segmenter"),
], { options: "SegmenterOptions" });

close();
await Bun.write(new URL("./src/intl.rs", import.meta.url), `${out.join("\n")}\n`);
