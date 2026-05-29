//! Minimal, dependency-free syntax highlighting for code blocks. A tiny generic
//! lexer (line/block comments, strings, numbers, keyword identifiers) wraps
//! tokens in `<span class="hl-…">`; every token and every gap is HTML-escaped,
//! and an unknown language falls back to plain escaped text. It is pure Rust, so
//! it runs in both the API host and the CRM wasm through the shared renderer —
//! the public site and the CRM preview colorize identically. Intentionally
//! basic (keyword/string/comment/number); extend the language table as needed.

use crate::escape_text;

/// Lexical shape of a language: its keyword set and comment/string delimiters.
struct Lang {
    keywords: &'static [&'static str],
    line_comment: &'static str,
    block: Option<(&'static str, &'static str)>,
    strings: &'static [char],
}

/// Highlight `code` for `language` into escaped HTML with `<span>` tokens, or
/// plain escaped text when the language is unknown.
pub(crate) fn highlight(language: &str, code: &str) -> String {
    let Some(lang) = lang_for(language) else {
        return escape_text(code);
    };
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::with_capacity(code.len() + 16);
    let mut plain = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Line comment → end of line.
        if !lang.line_comment.is_empty() && at(&chars, i, lang.line_comment) {
            flush(&mut out, &mut plain);
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            span(&mut out, "hl-com", &slice(&chars, start, i));
            continue;
        }
        // Block comment → closing delimiter (inclusive).
        if let Some((open, close)) = lang.block {
            if at(&chars, i, open) {
                flush(&mut out, &mut plain);
                let start = i;
                i += open.chars().count();
                while i < chars.len() && !at(&chars, i, close) {
                    i += 1;
                }
                if i < chars.len() {
                    i += close.chars().count();
                }
                span(&mut out, "hl-com", &slice(&chars, start, i));
                continue;
            }
        }
        // String → matching delimiter, honoring backslash escapes.
        if lang.strings.contains(&c) {
            flush(&mut out, &mut plain);
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == c {
                    i += 1;
                    break;
                }
                i += 1;
            }
            i = i.min(chars.len());
            span(&mut out, "hl-str", &slice(&chars, start, i));
            continue;
        }
        // Number.
        if c.is_ascii_digit() {
            flush(&mut out, &mut plain);
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            span(&mut out, "hl-num", &slice(&chars, start, i));
            continue;
        }
        // Identifier — a keyword gets a span, anything else is plain text.
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word = slice(&chars, start, i);
            if lang.keywords.contains(&word.as_str()) {
                flush(&mut out, &mut plain);
                span(&mut out, "hl-kw", &word);
            } else {
                plain.push_str(&word);
            }
            continue;
        }
        plain.push(c);
        i += 1;
    }
    flush(&mut out, &mut plain);
    out
}

fn slice(chars: &[char], start: usize, end: usize) -> String {
    chars[start..end].iter().collect()
}

/// True if `pat` (non-empty) occurs at `chars[i..]`.
fn at(chars: &[char], i: usize, pat: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    i + p.len() <= chars.len() && chars[i..i + p.len()] == p[..]
}

fn flush(out: &mut String, plain: &mut String) {
    if !plain.is_empty() {
        out.push_str(&escape_text(plain));
        plain.clear();
    }
}

fn span(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    out.push_str(&escape_text(text));
    out.push_str("</span>");
}

fn lang_for(language: &str) -> Option<&'static Lang> {
    match language.to_ascii_lowercase().as_str() {
        "rust" | "rs" => Some(&RUST),
        "js" | "javascript" | "ts" | "typescript" | "jsx" | "tsx" => Some(&JS),
        "python" | "py" => Some(&PYTHON),
        "go" | "golang" => Some(&GO),
        "bash" | "sh" | "shell" | "zsh" => Some(&BASH),
        "c" | "cpp" | "c++" | "h" | "hpp" => Some(&C),
        "json" => Some(&JSON),
        _ => None,
    }
}

static RUST: Lang = Lang {
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true",
        "type", "unsafe", "use", "where", "while",
    ],
    line_comment: "//",
    block: Some(("/*", "*/")),
    strings: &['"'],
};

static JS: Lang = Lang {
    keywords: &[
        "async", "await", "break", "case", "catch", "class", "const", "continue", "debugger",
        "default", "delete", "do", "else", "export", "extends", "false", "finally", "for",
        "function", "if", "import", "in", "instanceof", "let", "new", "null", "of", "return",
        "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while",
        "yield",
    ],
    line_comment: "//",
    block: Some(("/*", "*/")),
    strings: &['"', '\'', '`'],
};

static PYTHON: Lang = Lang {
    keywords: &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
        "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True", "try", "while",
        "with", "yield",
    ],
    line_comment: "#",
    block: None,
    strings: &['"', '\''],
};

static GO: Lang = Lang {
    keywords: &[
        "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough",
        "false", "for", "func", "go", "goto", "if", "import", "interface", "map", "nil", "package",
        "range", "return", "select", "struct", "switch", "true", "type", "var",
    ],
    line_comment: "//",
    block: Some(("/*", "*/")),
    strings: &['"', '`'],
};

static BASH: Lang = Lang {
    keywords: &[
        "case", "do", "done", "echo", "elif", "else", "esac", "export", "fi", "for", "function",
        "if", "in", "local", "return", "then", "while",
    ],
    line_comment: "#",
    block: None,
    strings: &['"', '\''],
};

static C: Lang = Lang {
    keywords: &[
        "auto", "bool", "break", "case", "char", "const", "continue", "default", "do", "double",
        "else", "enum", "extern", "false", "float", "for", "goto", "if", "int", "long", "register",
        "return", "short", "signed", "sizeof", "static", "struct", "switch", "true", "typedef",
        "union", "unsigned", "void", "volatile", "while",
    ],
    line_comment: "//",
    block: Some(("/*", "*/")),
    strings: &['"', '\''],
};

static JSON: Lang = Lang {
    keywords: &["true", "false", "null"],
    line_comment: "",
    block: None,
    strings: &['"'],
};
