//! Each JSX element's children as React is given them (#1807): a layout of
//! the same program keeps them, each text node whole, as a line broken at a
//! text's edge space would split one into two, `"Clicked "` into
//! `"Clicked"` and `" "`, the same markup and another layout in a browser.
//! JSX text is read by JSX's own rules, as Babel's
//! `cleanJSXElementLiteralChild` reads it: each line's tabs a space, the
//! space at a line's start but the first's and at its end but the last's
//! dropped, its lines of text joined by a space, a text of none no child.

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXChild, JSXElement, JSXExpression, JSXFragment};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// A child as React is given it: text, of JSX text or a string in braces
/// alike, or another node, an element or a value of another kind.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Child {
    Text(String),
    Node,
}

/// Each element's and fragment's children, in the order a visit meets them,
/// of `code`; `None` where it isn't JS this reads.
pub(super) fn shape(code: &str, source_type: SourceType) -> Option<Vec<Vec<Child>>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, source_type).parse();
    if parsed.diagnostics.errors().next().is_some() {
        return None;
    }
    let mut shapes = Shapes(Vec::new());
    shapes.visit_program(&parsed.program);
    Some(shapes.0)
}

struct Shapes(Vec<Vec<Child>>);

impl Shapes {
    fn children(&mut self, children: &[JSXChild<'_>]) {
        let mut shape = Vec::new();
        for child in children {
            match child {
                JSXChild::Text(text) => shape.extend(cleaned(&text.value).map(Child::Text)),
                JSXChild::ExpressionContainer(container) => match &container.expression {
                    JSXExpression::EmptyExpression(_) => {}
                    JSXExpression::StringLiteral(string) => shape.push(Child::Text(string.value.to_string())),
                    _ => shape.push(Child::Node),
                },
                _ => shape.push(Child::Node),
            }
        }
        self.0.push(shape);
    }
}

impl<'a> Visit<'a> for Shapes {
    fn visit_jsx_element(&mut self, element: &JSXElement<'a>) {
        self.children(&element.children);
        walk::walk_jsx_element(self, element);
    }

    fn visit_jsx_fragment(&mut self, fragment: &JSXFragment<'a>) {
        self.children(&fragment.children);
        walk::walk_jsx_fragment(self, fragment);
    }
}

/// JSX text as JSX reads it, `None` of what's no child.
fn cleaned(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split("\r\n").flat_map(|line| line.split(['\n', '\r'])).collect();
    // Babel's `lastNonEmptyLine`, the first line's where none has text.
    let last_text = lines
        .iter()
        .rposition(|line| line.contains(|c| c != ' ' && c != '\t'))
        .unwrap_or(0);
    let mut cleaned = String::new();
    for (i, line) in lines.iter().enumerate() {
        let line = line.replace('\t', " ");
        let line = if i == 0 {
            &line[..]
        } else {
            line.trim_start_matches(' ')
        };
        let line = if i + 1 == lines.len() {
            line
        } else {
            line.trim_end_matches(' ')
        };
        if !line.is_empty() {
            cleaned.push_str(line);
            if i != last_text {
                cleaned.push(' ');
            }
        }
    }
    (!cleaned.is_empty()).then_some(cleaned)
}

#[cfg(test)]
mod tests {
    use super::{Child, cleaned, shape};
    use oxc_span::SourceType;

    #[test]
    fn jsx_text_as_jsx_reads_it() {
        assert_eq!(cleaned("Clicked "), Some("Clicked ".to_string()));
        assert_eq!(cleaned("a\n      b"), Some("a b".to_string()));
        assert_eq!(cleaned("\n      "), None);
        assert_eq!(cleaned("\ta"), Some(" a".to_string()));
        assert_eq!(cleaned(" "), Some(" ".to_string()));
    }

    #[test]
    fn a_text_split_at_a_line_break_is_two_children() {
        let one = shape("<p>Clicked {count}</p>;", SourceType::jsx()).expect("JSX");
        let two = shape("<p>\n  Clicked{\" \"}\n  {count}\n</p>;", SourceType::jsx()).expect("JSX");
        assert_eq!(one, vec![vec![Child::Text("Clicked ".into()), Child::Node]]);
        assert_ne!(one, two);
    }
}
