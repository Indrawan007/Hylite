//! Read-only, dependency-free math subset for the native Preview.
//!
//! This is a small TeX-like expression parser, not a layout engine or a KaTeX
//! implementation. Accepted structures are linearized to Unicode; an unknown
//! command or malformed structure rejects the *whole* expression so the Preview
//! can show the original source with an explicit fallback marker.

/// Bound work on an individual formula before tokenizing.
pub const MAX_SOURCE_BYTES: usize = 2048;
/// Nested braces and `\\left … \\right` groups share one recursion limit.
pub const MAX_NESTING: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Empty,
    TooLong,
    TooDeep,
    UnknownCommand(String),
    Unbalanced,
    UnexpectedRight,
    MisplacedScript,
    DuplicateScript(char),
}

impl Error {
    /// Stable, user-visible explanation for an expression kept as source.
    pub fn reason(&self) -> String {
        match self {
            Self::Empty => "empty expression".into(),
            Self::TooLong => format!("longer than {MAX_SOURCE_BYTES} bytes"),
            Self::TooDeep => format!("nesting exceeds {MAX_NESTING} groups"),
            Self::UnknownCommand(name) => format!("unsupported command \\{name}"),
            Self::Unbalanced => "unbalanced group or missing argument".into(),
            Self::UnexpectedRight => "\\right has no matching \\left".into(),
            Self::MisplacedScript => "superscript or subscript has no base".into(),
            Self::DuplicateScript(operator) => format!("duplicate {operator} script"),
        }
    }
}

/// Cached result attached to one Preview run. `source` retains the delimited
/// Markdown spelling for PDF/source fallbacks; no reverse glyph mapping is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub display: bool,
    pub source: String,
    pub error: Option<Error>,
}

impl Projection {
    pub fn new(expression: &str, source: String, display: bool) -> (Self, String) {
        match render(expression) {
            Ok(text) => (
                Self {
                    display,
                    source,
                    error: None,
                },
                text,
            ),
            Err(error) => {
                let visible = format!("⟦math source fallback: {source} — {}⟧", error.reason());
                (
                    Self {
                        display,
                        source,
                        error: Some(error),
                    },
                    visible,
                )
            }
        }
    }
}

/// A parsed expression node. This is intentionally not a full TeX AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Row(Vec<Node>),
    Text(String),
    Fraction(Box<Node>, Box<Node>),
    SquareRoot(Box<Node>),
    Scripts {
        base: Box<Node>,
        subscript: Option<Box<Node>>,
        superscript: Option<Box<Node>>,
    },
    Delimited {
        left: String,
        body: Box<Node>,
        right: String,
    },
}

/// Parse and linearize one expression. The caller decides how to present errors.
pub fn render(source: &str) -> Result<String, Error> {
    Ok(to_text(&parse(source)?))
}

pub fn parse(source: &str) -> Result<Node, Error> {
    if source.trim().is_empty() {
        return Err(Error::Empty);
    }
    if source.len() > MAX_SOURCE_BYTES {
        return Err(Error::TooLong);
    }

    let mut parser = Parser {
        chars: source.chars().collect(),
        at: 0,
    };
    parser.row(Stop::End, 0)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    End,
    Brace,
    Right,
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.at += 1;
        Some(character)
    }

    fn row(&mut self, stop: Stop, depth: usize) -> Result<Node, Error> {
        if depth > MAX_NESTING {
            return Err(Error::TooDeep);
        }

        let mut nodes = Vec::new();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => match stop {
                    Stop::End => break,
                    Stop::Brace | Stop::Right => return Err(Error::Unbalanced),
                },
                Some('}') if stop == Stop::Brace => break,
                Some('}') => return Err(Error::Unbalanced),
                Some('\\') if self.next_command_is("right") => match stop {
                    Stop::Right => break,
                    Stop::End | Stop::Brace => return Err(Error::UnexpectedRight),
                },
                Some('^') | Some('_') => {
                    let operator = self.next().unwrap_or_default();
                    flush(&mut nodes, &mut text);
                    let base = take_script_base(&mut nodes)?;
                    let script = self.script_argument(depth)?;
                    nodes.push(attach_script(base, operator, script)?);
                }
                Some('{') => {
                    self.next();
                    flush(&mut nodes, &mut text);
                    nodes.push(self.group(depth)?);
                }
                Some('\\') => {
                    flush(&mut nodes, &mut text);
                    nodes.push(self.control_sequence(depth)?);
                }
                Some(character) => {
                    self.next();
                    text.push(character);
                }
            }
        }
        flush(&mut nodes, &mut text);
        Ok(Node::Row(nodes))
    }

    fn group(&mut self, depth: usize) -> Result<Node, Error> {
        if depth >= MAX_NESTING {
            return Err(Error::TooDeep);
        }
        let body = self.row(Stop::Brace, depth + 1)?;
        if self.next() != Some('}') {
            return Err(Error::Unbalanced);
        }
        Ok(body)
    }

    fn script_argument(&mut self, depth: usize) -> Result<Node, Error> {
        self.skip_spaces();
        match self.peek() {
            Some('{') => {
                self.next();
                self.group(depth)
            }
            Some('}') | None => Err(Error::Unbalanced),
            Some('\\') => self.control_sequence(depth),
            Some(character) => {
                self.next();
                Ok(Node::Text(character.to_string()))
            }
        }
    }

    fn command_argument(&mut self, depth: usize) -> Result<Node, Error> {
        self.skip_spaces();
        match self.peek() {
            Some('{') => {
                self.next();
                self.group(depth)
            }
            Some('}') | None => Err(Error::Unbalanced),
            Some('\\') => self.control_sequence(depth),
            Some(character) => {
                self.next();
                Ok(Node::Text(character.to_string()))
            }
        }
    }

    fn control_sequence(&mut self, depth: usize) -> Result<Node, Error> {
        debug_assert_eq!(self.peek(), Some('\\'));
        self.next();
        match self.next() {
            Some(character) if character.is_ascii_alphabetic() => {
                let mut name = String::from(character);
                while matches!(self.peek(), Some(next) if next.is_ascii_alphabetic()) {
                    if let Some(next) = self.next() {
                        name.push(next);
                    }
                }
                self.command(&name, depth)
            }
            Some('\\') => Ok(Node::Text("\n".into())),
            Some(character) => match character {
                '{' | '}' | '$' | '%' | '#' | '&' | '_' | '^' | '~' => {
                    Ok(Node::Text(character.to_string()))
                }
                ' ' => Ok(Node::Text(" ".into())),
                ',' | ';' | ':' => Ok(Node::Text(" ".into())),
                '!' => Ok(Node::Text(String::new())),
                _ => Err(Error::UnknownCommand(character.to_string())),
            },
            None => Err(Error::Unbalanced),
        }
    }

    fn command(&mut self, name: &str, depth: usize) -> Result<Node, Error> {
        if let Some(symbol) = symbol(name) {
            return Ok(Node::Text(symbol.into()));
        }
        match name {
            "frac" => Ok(Node::Fraction(
                Box::new(self.command_argument(depth)?),
                Box::new(self.command_argument(depth)?),
            )),
            "sqrt" => Ok(Node::SquareRoot(Box::new(self.command_argument(depth)?))),
            "quad" => Ok(Node::Text("  ".into())),
            "qquad" => Ok(Node::Text("    ".into())),
            "left" => self.delimited(depth),
            "right" => Err(Error::UnexpectedRight),
            _ => Err(Error::UnknownCommand(name.into())),
        }
    }

    fn delimited(&mut self, depth: usize) -> Result<Node, Error> {
        if depth >= MAX_NESTING {
            return Err(Error::TooDeep);
        }
        self.skip_spaces();
        let left = self.delimiter()?;
        let body = self.row(Stop::Right, depth + 1)?;
        if !self.next_command_is("right") {
            return Err(Error::Unbalanced);
        }
        self.consume_command("right")?;
        self.skip_spaces();
        let right = self.delimiter()?;
        Ok(Node::Delimited {
            left,
            body: Box::new(body),
            right,
        })
    }

    fn delimiter(&mut self) -> Result<String, Error> {
        match self.next() {
            Some('\\') => match self.next() {
                Some('{') => Ok("{".into()),
                Some('}') => Ok("}".into()),
                Some(character) if character.is_ascii_alphabetic() => {
                    let mut name = String::from(character);
                    while matches!(self.peek(), Some(next) if next.is_ascii_alphabetic()) {
                        if let Some(next) = self.next() {
                            name.push(next);
                        }
                    }
                    match name.as_str() {
                        "langle" => Ok("⟨".into()),
                        "rangle" => Ok("⟩".into()),
                        "vert" => Ok("|".into()),
                        "Vert" => Ok("‖".into()),
                        _ => Err(Error::UnknownCommand(name)),
                    }
                }
                _ => Err(Error::Unbalanced),
            },
            Some(character @ ('.' | '(' | ')' | '[' | ']' | '|' | '/' | '<' | '>')) => {
                Ok(if character == '.' {
                    String::new()
                } else {
                    character.to_string()
                })
            }
            _ => Err(Error::Unbalanced),
        }
    }

    fn next_command_is(&self, expected: &str) -> bool {
        let Some('\\') = self.peek() else {
            return false;
        };
        let start = self.at + 1;
        let mut end = start;
        while matches!(self.chars.get(end), Some(character) if character.is_ascii_alphabetic()) {
            end += 1;
        }
        end > start && self.chars[start..end].iter().collect::<String>() == expected
    }

    fn consume_command(&mut self, expected: &str) -> Result<(), Error> {
        if !self.next_command_is(expected) {
            return Err(Error::Unbalanced);
        }
        self.next();
        self.next();
        for _ in expected.chars().skip(1) {
            self.next();
        }
        Ok(())
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
            self.next();
        }
    }
}

fn take_script_base(nodes: &mut Vec<Node>) -> Result<Node, Error> {
    match nodes.pop().ok_or(Error::MisplacedScript)? {
        Node::Text(text) => {
            let text = text.trim_end_matches(char::is_whitespace);
            let (index, last) = text
                .char_indices()
                .next_back()
                .ok_or(Error::MisplacedScript)?;
            if index > 0 {
                nodes.push(Node::Text(text[..index].to_owned()));
            }
            Ok(Node::Text(last.to_string()))
        }
        base => Ok(base),
    }
}

fn attach_script(base: Node, operator: char, script: Node) -> Result<Node, Error> {
    let (base, mut subscript, mut superscript) = match base {
        Node::Scripts {
            base,
            subscript,
            superscript,
        } => (*base, subscript, superscript),
        base => (base, None, None),
    };
    let slot = if operator == '^' {
        &mut superscript
    } else {
        &mut subscript
    };
    if slot.is_some() {
        return Err(Error::DuplicateScript(operator));
    }
    *slot = Some(Box::new(script));
    Ok(Node::Scripts {
        base: Box::new(base),
        subscript,
        superscript,
    })
}

fn flush(nodes: &mut Vec<Node>, text: &mut String) {
    if !text.is_empty() {
        nodes.push(Node::Text(std::mem::take(text)));
    }
}

fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" | "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" | "vartheta" => "θ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" | "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "times" => "×",
        "cdot" => "·",
        "pm" => "±",
        "mp" => "∓",
        "div" => "÷",
        "le" | "leq" => "≤",
        "ge" | "geq" => "≥",
        "ne" | "neq" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "to" | "rightarrow" => "→",
        "leftarrow" => "←",
        "Rightarrow" => "⇒",
        "Leftrightarrow" => "⇔",
        "infty" => "∞",
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "partial" => "∂",
        "in" => "∈",
        "notin" => "∉",
        "subset" => "⊂",
        "subseteq" => "⊆",
        "supset" => "⊃",
        "supseteq" => "⊇",
        "cup" => "∪",
        "cap" => "∩",
        "forall" => "∀",
        "exists" => "∃",
        "nabla" => "∇",
        "angle" => "∠",
        "degree" => "°",
        "prime" => "′",
        "ldots" | "dots" => "…",
        "cdots" => "⋯",
        "log" => "log",
        "ln" => "ln",
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "lim" => "lim",
        "max" => "max",
        "min" => "min",
        _ => return None,
    })
}

/// Linearize a parsed tree. This is readable native text, not two-dimensional
/// typesetting; complex fractions/scripts use parentheses rather than layout.
pub fn to_text(node: &Node) -> String {
    match node {
        Node::Row(nodes) => nodes.iter().map(to_text).collect(),
        Node::Text(text) => text.clone(),
        Node::Fraction(numerator, denominator) => {
            format!("{}⁄{}", part(numerator), part(denominator))
        }
        Node::SquareRoot(body) => format!("√{}", part(body)),
        Node::Scripts {
            base,
            subscript,
            superscript,
        } => {
            let mut text = to_text(base);
            if let Some(subscript) = subscript {
                let body = to_text(subscript);
                text.push_str(&subscript_text(&body).unwrap_or_else(|| format!("_({body})")));
            }
            if let Some(superscript) = superscript {
                let body = to_text(superscript);
                text.push_str(&superscript_text(&body).unwrap_or_else(|| format!("^({body})")));
            }
            text
        }
        Node::Delimited { left, body, right } => {
            format!("{left}{}{right}", to_text(body))
        }
    }
}

fn part(node: &Node) -> String {
    let text = to_text(node);
    if !text.is_empty() && text.chars().all(char::is_alphanumeric) {
        text
    } else {
        format!("({text})")
    }
}

fn map_chars(text: &str, table: &[(char, char)]) -> Option<String> {
    let mut mapped = String::with_capacity(text.len());
    for character in text.chars() {
        mapped.push(table.iter().find(|(from, _)| *from == character)?.1);
    }
    Some(mapped)
}

fn superscript_text(text: &str) -> Option<String> {
    map_chars(
        text,
        &[
            ('0', '⁰'),
            ('1', '¹'),
            ('2', '²'),
            ('3', '³'),
            ('4', '⁴'),
            ('5', '⁵'),
            ('6', '⁶'),
            ('7', '⁷'),
            ('8', '⁸'),
            ('9', '⁹'),
            ('+', '⁺'),
            ('-', '⁻'),
            ('=', '⁼'),
            ('(', '⁽'),
            (')', '⁾'),
            ('a', 'ᵃ'),
            ('b', 'ᵇ'),
            ('c', 'ᶜ'),
            ('d', 'ᵈ'),
            ('e', 'ᵉ'),
            ('f', 'ᶠ'),
            ('g', 'ᵍ'),
            ('h', 'ʰ'),
            ('i', 'ⁱ'),
            ('j', 'ʲ'),
            ('k', 'ᵏ'),
            ('l', 'ˡ'),
            ('m', 'ᵐ'),
            ('n', 'ⁿ'),
            ('o', 'ᵒ'),
            ('p', 'ᵖ'),
            ('r', 'ʳ'),
            ('s', 'ˢ'),
            ('t', 'ᵗ'),
            ('u', 'ᵘ'),
            ('v', 'ᵛ'),
            ('w', 'ʷ'),
            ('x', 'ˣ'),
            ('y', 'ʸ'),
            ('z', 'ᶻ'),
        ],
    )
}

fn subscript_text(text: &str) -> Option<String> {
    map_chars(
        text,
        &[
            ('0', '₀'),
            ('1', '₁'),
            ('2', '₂'),
            ('3', '₃'),
            ('4', '₄'),
            ('5', '₅'),
            ('6', '₆'),
            ('7', '₇'),
            ('8', '₈'),
            ('9', '₉'),
            ('+', '₊'),
            ('-', '₋'),
            ('=', '₌'),
            ('(', '₍'),
            (')', '₎'),
            ('a', 'ₐ'),
            ('e', 'ₑ'),
            ('h', 'ₕ'),
            ('i', 'ᵢ'),
            ('j', 'ⱼ'),
            ('k', 'ₖ'),
            ('l', 'ₗ'),
            ('m', 'ₘ'),
            ('n', 'ₙ'),
            ('o', 'ₒ'),
            ('p', 'ₚ'),
            ('r', 'ᵣ'),
            ('s', 'ₛ'),
            ('t', 'ₜ'),
            ('u', 'ᵤ'),
            ('v', 'ᵥ'),
            ('x', 'ₓ'),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(source: &str) -> String {
        render(source).unwrap_or_else(|error| panic!("{source}: {}", error.reason()))
    }

    #[test]
    fn greek_letters_and_common_operators_linearize() {
        assert_eq!(text("\\alpha + \\beta \\le \\pi \\cdot 2"), "α + β ≤ π · 2");
        assert_eq!(text("\\sin x \\to \\infty"), "sin x → ∞");
        assert_eq!(text("\\sum_{i=1}^{n} i"), "∑ᵢ₌₁ⁿ i");
        assert_eq!(text("\\int_0^\\infty e^{-x} dx"), "∫₀^(∞) e⁻ˣ dx");
    }

    #[test]
    fn fractions_and_roots_keep_compound_arguments_visible() {
        assert_eq!(text("\\frac{1}{2}"), "1⁄2");
        assert_eq!(text("\\frac{a+b}{c}"), "(a+b)⁄c");
        assert_eq!(text("\\frac{\\alpha}{2}"), "α⁄2");
        assert_eq!(text("\\sqrt{2}"), "√2");
        assert_eq!(text("\\sqrt{x+1}"), "√(x+1)");
    }

    #[test]
    fn superscripts_and_subscripts_attach_to_the_previous_atom_in_either_order() {
        assert_eq!(text("x^2"), "x²");
        assert_eq!(text("a_1"), "a₁");
        assert_eq!(text("x^{n+1}"), "xⁿ⁺¹");
        assert_eq!(text("x_i^2"), "xᵢ²");
        assert_eq!(text("x^2_i"), "xᵢ²");
        assert_eq!(text("a+b^2"), "a+b²");
        assert_eq!(text("x^{\\alpha}"), "x^(α)");
    }

    #[test]
    fn paired_delimiters_and_escaped_characters_are_literal() {
        assert_eq!(text("\\left( a+b \\right)"), "( a+b )");
        assert_eq!(text("\\left. x \\right|"), " x |");
        assert_eq!(text("\\{a\\}"), "{a}");
        assert_eq!(text("50\\%"), "50%");
        assert_eq!(text("a \\, b"), "a   b");
        assert_eq!(text("a \\\\ b"), "a \n b");
    }

    #[test]
    fn unknown_commands_reject_the_whole_expression() {
        assert_eq!(parse("\\foo{x}"), Err(Error::UnknownCommand("foo".into())));
        assert_eq!(
            parse("a + \\bar b"),
            Err(Error::UnknownCommand("bar".into()))
        );
        assert_eq!(
            parse("\\foo{x}").unwrap_err().reason(),
            "unsupported command \\foo"
        );
    }

    #[test]
    fn malformed_structure_has_a_bounded_explicit_error() {
        for (source, error) in [
            ("   ", Error::Empty),
            ("{a", Error::Unbalanced),
            ("a}", Error::Unbalanced),
            ("x^", Error::Unbalanced),
            ("^x", Error::MisplacedScript),
            ("x^2^3", Error::DuplicateScript('^')),
            ("\\frac{1}", Error::Unbalanced),
            ("\\left(x", Error::Unbalanced),
            ("\\right)", Error::UnexpectedRight),
        ] {
            assert_eq!(parse(source), Err(error), "{source}");
        }
        let too_deep = format!(
            "{}x{}",
            "{".repeat(MAX_NESTING + 1),
            "}".repeat(MAX_NESTING + 1)
        );
        assert_eq!(parse(&too_deep), Err(Error::TooDeep));
    }

    #[test]
    fn preview_projection_marks_failure_and_retains_source_for_export() {
        let source = "$\\unknown{x}$".to_owned();
        let (projection, visible) = Projection::new("\\unknown{x}", source.clone(), false);
        assert_eq!(projection.source, source);
        assert!(projection.error.is_some());
        assert!(visible.contains("math source fallback"));
        assert!(visible.contains("$\\unknown{x}$"));

        let (projection, visible) = Projection::new("x^2", "$x^2$".into(), false);
        assert!(projection.error.is_none());
        assert_eq!(visible, "x²");
    }

    #[test]
    fn oversized_source_is_refused_before_tokenizing() {
        let long = "1".repeat(MAX_SOURCE_BYTES + 1);
        assert_eq!(parse(&long), Err(Error::TooLong));
        assert_eq!(
            parse(&long).unwrap_err().reason(),
            format!("longer than {MAX_SOURCE_BYTES} bytes")
        );
    }

    #[test]
    fn every_refusal_path_returns_an_error_without_panicking() {
        for source in [
            "\\", "{", "}", "^", "_", "\\frac", "\\sqrt", "\\left(", "\\foo",
        ] {
            let error = parse(source).unwrap_err();
            assert!(!error.reason().is_empty(), "{source}");
        }
    }
}
