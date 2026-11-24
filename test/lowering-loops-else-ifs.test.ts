// What the compiler writes of a loop that ends in an `if` and its `else
// if`s, as react.dev's llms.txt and CodeBlock write theirs: the chain
// kept one past a written `continue`, not a `continue` of its own. Each
// test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

// An \`else if\` the program wrote at a loop's end, after an \`if\` it ended
// with a \`continue\`, stays an \`else if\` from MIR: not a \`continue\` of
// its own and an \`if\` after, as react.dev's llms.txt sections its routes.
test("an else-if at a loop's end from MIR stays one past a written continue", async () => {
  const dir = fixture("mir-else-if-after-continue");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, dead_code)]
struct RouteItem {
    title: Option<String>,
    path: Option<String>,
    routes: Option<Vec<RouteItem>>,
    has_section_header: Option<bool>,
    section_header: Option<String>,
}

struct Sidebar {
    title: String,
    routes: Vec<RouteItem>,
}

struct Page {
    title: String,
    url: String,
}

struct SubGroup {
    heading: String,
    pages: Vec<Page>,
}

struct Section {
    heading: Option<String>,
    pages: Vec<Page>,
    sub_groups: Vec<SubGroup>,
}

// Clean up section header names (remove version placeholders)
fn cleanSectionHeader(header: &str) -> String { header.trim().to_string() }
pub fn extractSectionedRoutes(routes: &[RouteItem], baseUrl: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut currentSection: Option<Section> = None;

    for route in routes {
        // Skip external links
        if route.path.as_ref().is_some_and(|path| path.starts_with("http")) {
            continue;
        }

        // Start a new section when we hit a section header
        if route.has_section_header == Some(true)
            && let Some(sectionHeader) = &route.section_header
        {
            if let Some(section) = currentSection {
                sections.push(section);
            }
            currentSection = Some(Section {
                heading: Some(cleanSectionHeader(sectionHeader)),
                pages: Vec::new(),
                sub_groups: Vec::new(),
            });
            continue;
        }

        // If no section started yet, skip
        let Some(currentSection) = &mut currentSection else {
            continue;
        };

        // Route with children - create a sub-group
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            let mut subGroup = SubGroup {
                heading: title.clone(),
                pages: Vec::new(),
            };

            // Include parent page if it has a path
            if let Some(path) = &route.path {
                subGroup.pages.push(Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                });
            }

            // Add child pages
            for child in children {
                if let Some(childTitle) = &child.title
                    && let Some(childPath) = &child.path
                    && !childPath.starts_with("http")
                {
                    subGroup.pages.push(Page {
                        title: childTitle.clone(),
                        url: format!("{baseUrl}{childPath}.md"),
                    });
                }
            }

            if !subGroup.pages.is_empty() {
                currentSection.sub_groups.push(subGroup);
            }
        }
        // Single page without children
        else if let Some(title) = &route.title
            && let Some(path) = &route.path
        {
            currentSection.pages.push(Page {
                title: title.clone(),
                url: format!("{baseUrl}{path}.md"),
            });
        }
    }

    // Don't forget the last section
    if let Some(section) = currentSection {
        sections.push(section);
    }

    sections
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = withoutBoundary(readFileSync(join(dir, "mir.js"), "utf8"));
  // Its comment before the `else`, as upstream has it (ADR 0444, amended).
  expect(js).toContain("      }\n    }\n    // Single page without children\n    else if (route.title != null && route.path != null) {\n");
  // Nor a \`continue\` where the body ends anyway.
  expect(js).not.toMatch(/\n      continue;\n    } else if/);
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { hasSectionHeader: true, sectionHeader: "A" },
    { title: "t", path: "/p" },
    { title: "g", routes: [{ title: "c", path: "/c" }] },
  ];
  expect(lib.extractSectionedRoutes(routes, "x").map((s: { heading: string; pages: unknown[]; subGroups: unknown[] }) => [s.heading, s.pages.length, s.subGroups.length])).toEqual([["A", 1, 1]]);
});

// react.dev's llms.txt groups its routes: an `else if` the loop's body ends
// with, after an `if` that doesn't end in a `continue`, stays an `else if`,
// not a `continue` MIR's block for it makes and an `if` after.
test("react.dev's llms.txt groups its routes with an else-if at the loop's end", async () => {
  const dir = fixture("mir-grouped-routes");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, dead_code)]
// Data JS gives, its fields its own: no type only Rust makes (ADR 1196).
pub struct RouteItem {
    pub title: Option<String>,
    pub path: Option<String>,
    pub routes: Option<Vec<RouteItem>>,
    pub has_section_header: Option<bool>,
}

pub struct Page {
    title: String,
    url: String,
}

pub struct SubGroup {
    heading: String,
    pages: Vec<Page>,
}

pub fn extractGroupedRoutes(routes: &[RouteItem], baseUrl: &str) -> Vec<SubGroup> {
    let mut groups: Vec<SubGroup> = Vec::new();

    for route in routes {
        // Skip section headers
        if route.has_section_header == Some(true) {
            continue;
        }

        // Skip external links
        if route.path.as_ref().is_some_and(|path| path.starts_with("http")) {
            continue;
        }

        // Route with children - create a group
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            let mut pages: Vec<Page> = Vec::new();

            // Include parent page if it has a path
            if let Some(path) = &route.path {
                pages.push(Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                });
            }

            // Add child pages
            for child in children {
                if let Some(childTitle) = &child.title
                    && let Some(childPath) = &child.path
                    && !childPath.starts_with("http")
                {
                    pages.push(Page {
                        title: childTitle.clone(),
                        url: format!("{baseUrl}{childPath}.md"),
                    });
                }
            }

            if !pages.is_empty() {
                groups.push(SubGroup {
                    heading: title.clone(),
                    pages,
                });
            }
        }
        // Single page without children - group under its own heading
        else if let Some(title) = &route.title
            && let Some(path) = &route.path
        {
            groups.push(SubGroup {
                heading: title.clone(),
                pages: vec![Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                }],
            });
        }
    }

    groups
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = withoutBoundary(readFileSync(join(dir, "mir.js"), "utf8"));
  expect(js).toContain("        groups.push({\n          heading: route.title,\n          pages,\n        });\n      }\n    }\n    // Single page without children - group under its own heading\n    else if (route.title != null && route.path != null) {\n");
  expect(js).not.toContain("\n    if (route.title != null && route.path != null) {");
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { hasSectionHeader: true, title: "H", path: "/h" },
    { title: "E", path: "https://e" },
    { title: "G", path: "/g", routes: [{ title: "c", path: "/c" }, { title: "x", path: "http://x" }] },
    { title: "T", path: "/t" },
    { title: "N" },
  ];
  const grouped = lib.extractGroupedRoutes(routes, "b").map((g: { heading: string; pages: { url: string }[] }) => [g.heading, g.pages.map((p) => p.url)]);
  expect(grouped).toEqual([["G", ["b/g.md", "b/c.md"]], ["T", ["b/t.md"]]]);
});

// react.dev's CodeBlock places its inline decorators: a loop's body ending
// in an `if`'s branch, here its last insert past the `panic!`s' `if`s, ends
// with no `continue` MIR's block for it makes, which does nothing there.
test("react.dev's CodeBlock places its decorators with no continue at the loop's end", async () => {
  const dir = fixture("mir-decorator-ends");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;

pub struct InlineDecorator {
    pub line: u32,
    pub start_column: u32,
    pub end_column: u32,
    pub class_name: String,
}

pub fn ends(lines: &[&str], inline_decorators: Vec<InlineDecorator>) -> Vec<(u32, String)> {
    let mut decorator_starts = HashMap::new();
    let mut decorator_ends = HashMap::new();
    for decorator in inline_decorators {
        // Find where inline highlight starts and ends.
        let mut decorator_start = 0;
        for i in 0..decorator.line - 1 {
            decorator_start += lines[i as usize].len() as u32 + 1;
        }
        decorator_start += decorator.start_column;
        let decorator_end = decorator_start + (decorator.end_column - decorator.start_column);
        if decorator_starts.contains_key(&decorator_start) {
            panic!("Already opened decorator at {decorator_start}");
        }
        decorator_starts.insert(decorator_start, decorator.class_name.clone());
        if decorator_ends.contains_key(&decorator_end) {
            panic!("Already closed decorator at {decorator_end}");
        }
        decorator_ends.insert(decorator_end, decorator.class_name);
    }
    let mut out: Vec<(u32, String)> = decorator_ends.into_iter().collect();
    out.sort();
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = withoutBoundary(readFileSync(join(dir, "mir.js"), "utf8"));
  expect(js).toContain("    decoratorEnds.set(decoratorEnd, decorator.className);\n  }\n");
  expect(js).not.toContain("continue;");
  const lib = await import(join(dir, "mir.js"));
  const decorator = (line: number, startColumn: number, endColumn: number, className: string) => ({ line, startColumn, endColumn, className });
  expect(lib.ends(["ab", "cde"], [decorator(2, 1, 3, "k"), decorator(1, 0, 1, "s")])).toEqual([[1, "s"], [6, "k"]]);
  expect(() => lib.ends(["ab"], [decorator(1, 0, 1, "a"), decorator(1, 1, 1, "b")])).toThrow("Already closed decorator at 1");
});
