use std::fmt::Write;

use super::ParseError;

/// Render a single parse error as a rustc-style diagnostic, e.g.
///
/// ```text
/// error: Parse error: expected metadata property, got Ident("wobble")
///  --> demo.csf:4:3
///   |
/// 4 |   wobble 5
///   |   ^^^^^^
/// ```
///
/// `color` toggles ANSI escapes so callers can disable them when stdout isn't a
/// terminal.
pub fn render_diagnostic(error: &ParseError, filename: &str, source: &str, color: bool) -> String {
    let span = error.span;
    let line_idx = span.line() as usize;
    let col = span.col() as usize;

    let line_no = line_idx + 1;
    let col_no = col + 1;

    let src_line = source.lines().nth(line_idx).unwrap_or("");

    // Clamp the underline to the visible portion of the line so a stray length
    // can't run away, but always draw at least one caret.
    let remaining = src_line.chars().count().saturating_sub(col);
    let carets = (span.length() as usize).min(remaining.max(1)).max(1);

    let gutter = line_no.to_string().len();
    let pad = " ".repeat(gutter);

    let (red, blue, bold, reset) = if color {
        ("\x1b[1;31m", "\x1b[1;34m", "\x1b[1m", "\x1b[0m")
    } else {
        ("", "", "", "")
    };

    let mut out = String::new();
    writeln!(out, "{red}error{reset}{bold}: {}{reset}", error.kind).unwrap();
    writeln!(out, "{pad}{blue}-->{reset} {filename}:{line_no}:{col_no}").unwrap();
    writeln!(out, "{pad} {blue}|{reset}").unwrap();
    writeln!(out, "{blue}{line_no} |{reset} {src_line}").unwrap();
    writeln!(
        out,
        "{pad} {blue}|{reset} {caret_pad}{red}{carets}{reset}",
        caret_pad = " ".repeat(col),
        carets = "^".repeat(carets),
    )
    .unwrap();

    out
}
