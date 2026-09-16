//! ICU MessageFormat — the subset the catalogs use, parsed once at startup.
//!
//! Supported: `{name}`, `{name, number}`, `{name, plural, offset:n =0 {…} one {…} other {…}}`,
//! `{name, select, a {…} other {…}}`, `#` inside plural, apostrophe quoting
//! (`''` → `'`, `'{literal}'`). `other` is mandatory for plural and select; the parser rejects
//! messages without it, so a broken translation fails the catalog tests instead of a user.
//!
//! Not supported on purpose (add when a catalog needs it): `date`, `time`, `selectordinal`,
//! `spellout`, `choice`, number skeletons.

use std::collections::BTreeSet;
use std::fmt;

use crate::locale::{Language, Locale, PluralCategory};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Message(pub(crate) Vec<Part>);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Part {
    Text(String),
    /// `{name}` — string as is, numbers formatted for the locale.
    Argument(String),
    /// `{name, number}`
    Number(String),
    /// `#` inside a plural case: the plural number minus offset, locale formatted.
    Pound,
    Plural {
        argument: String,
        offset: i64,
        cases: Vec<(PluralKey, Message)>,
    },
    Select {
        argument: String,
        cases: Vec<(String, Message)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PluralKey {
    Exact(i64),
    Category(PluralCategory),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub position: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at char {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

/// Values a message can be rendered with.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
}

impl Value {
    fn as_number(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(x) => Some(*x),
            Value::Str(_) => None,
        }
    }

    fn render(&self, locale: Locale) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Int(n) => locale.format_integer(*n),
            Value::Float(x) => locale.format_decimal(*x),
        }
    }
}

/// Named arguments for one rendering. Small and ordered; no HashMap ceremony.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Args(Vec<(String, Value)>);

impl Args {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn str(mut self, name: &str, value: impl Into<String>) -> Self {
        self.0.push((name.to_string(), Value::Str(value.into())));
        self
    }

    pub fn int(mut self, name: &str, value: i64) -> Self {
        self.0.push((name.to_string(), Value::Int(value)));
        self
    }

    pub fn float(mut self, name: &str, value: f64) -> Self {
        self.0.push((name.to_string(), Value::Float(value)));
        self
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.0
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }
}

impl Message {
    pub fn parse(source: &str) -> Result<Message, ParseError> {
        let mut parser = Parser {
            chars: source.chars().collect(),
            pos: 0,
        };
        let message = parser.message(false, 0)?;
        if parser.pos < parser.chars.len() {
            return Err(parser.error("unmatched `}`"));
        }
        Ok(message)
    }

    /// Names of all arguments the message reads, nested cases included.
    /// Translations of one key must agree on this set.
    pub fn arguments(&self) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        self.collect_arguments(&mut names);
        names
    }

    fn collect_arguments(&self, names: &mut BTreeSet<String>) {
        for part in &self.0 {
            match part {
                Part::Text(_) | Part::Pound => {}
                Part::Argument(name) | Part::Number(name) => {
                    names.insert(name.clone());
                }
                Part::Plural {
                    argument, cases, ..
                } => {
                    names.insert(argument.clone());
                    for (_, case) in cases {
                        case.collect_arguments(names);
                    }
                }
                Part::Select { argument, cases } => {
                    names.insert(argument.clone());
                    for (_, case) in cases {
                        case.collect_arguments(names);
                    }
                }
            }
        }
    }

    pub fn render(&self, language: Language, locale: Locale, args: &Args) -> String {
        let mut out = String::new();
        self.render_into(&mut out, language, locale, args, None);
        out
    }

    fn render_into(
        &self,
        out: &mut String,
        language: Language,
        locale: Locale,
        args: &Args,
        pound: Option<f64>,
    ) {
        for part in &self.0 {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Argument(name) | Part::Number(name) => match args.get(name) {
                    Some(value) => out.push_str(&value.render(locale)),
                    // Missing argument is a programming error; keep it visible, never silent.
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                },
                Part::Pound => match pound {
                    Some(n) => out.push_str(&format_number(n, locale)),
                    None => out.push('#'),
                },
                Part::Plural {
                    argument,
                    offset,
                    cases,
                } => {
                    let Some(n) = args.get(argument).and_then(Value::as_number) else {
                        out.push('{');
                        out.push_str(argument);
                        out.push('}');
                        continue;
                    };
                    let shifted = n - *offset as f64;
                    let category = language.plural_category(shifted);
                    let case = cases
                        .iter()
                        .find(
                            |(key, _)| matches!(key, PluralKey::Exact(exact) if *exact as f64 == n),
                        )
                        .or_else(|| {
                            cases
                                .iter()
                                .find(|(key, _)| *key == PluralKey::Category(category))
                        })
                        .or_else(|| {
                            cases
                                .iter()
                                .find(|(key, _)| *key == PluralKey::Category(PluralCategory::Other))
                        });
                    if let Some((_, message)) = case {
                        message.render_into(out, language, locale, args, Some(shifted));
                    }
                }
                Part::Select { argument, cases } => {
                    let selector = match args.get(argument) {
                        Some(Value::Str(s)) => s.clone(),
                        Some(other) => other.render(locale),
                        None => String::new(),
                    };
                    let case = cases
                        .iter()
                        .find(|(key, _)| *key == selector)
                        .or_else(|| cases.iter().find(|(key, _)| key == "other"));
                    if let Some((_, message)) = case {
                        message.render_into(out, language, locale, args, pound);
                    }
                }
            }
        }
    }
}

fn format_number(n: f64, locale: Locale) -> String {
    if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
        locale.format_integer(n as i64)
    } else {
        locale.format_decimal(n)
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn error(&self, message: &str) -> ParseError {
        ParseError {
            position: self.pos,
            message: message.to_string(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), ParseError> {
        match self.bump() {
            Some(c) if c == expected => Ok(()),
            _ => Err(self.error(&format!("expected `{expected}`"))),
        }
    }

    fn word(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.error("expected a name"));
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn integer(&mut self) -> Result<i64, ParseError> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse().map_err(|_| self.error("expected an integer"))
    }

    /// Parse until `}` (when nested) or end of input.
    fn message(&mut self, in_plural: bool, depth: usize) -> Result<Message, ParseError> {
        let mut parts = Vec::new();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('}') => {
                    if depth == 0 {
                        return Err(self.error("unmatched `}`"));
                    }
                    break;
                }
                Some('{') => {
                    flush(&mut text, &mut parts);
                    self.pos += 1;
                    parts.push(self.argument(in_plural, depth)?);
                }
                Some('#') if in_plural => {
                    flush(&mut text, &mut parts);
                    self.pos += 1;
                    parts.push(Part::Pound);
                }
                Some('\'') => {
                    self.pos += 1;
                    self.quoted(in_plural, &mut text);
                }
                Some(c) => {
                    text.push(c);
                    self.pos += 1;
                }
            }
        }
        flush(&mut text, &mut parts);
        Ok(Message(parts))
    }

    /// After an apostrophe: `''` is a literal apostrophe; `'` before a syntax char starts a
    /// quoted literal that runs to the next single `'`; any other `'` is literal.
    fn quoted(&mut self, in_plural: bool, text: &mut String) {
        match self.peek() {
            Some('\'') => {
                self.pos += 1;
                text.push('\'');
            }
            Some(c) if c == '{' || c == '}' || (c == '#' && in_plural) => {
                while let Some(ch) = self.bump() {
                    if ch == '\'' {
                        if self.peek() == Some('\'') {
                            self.pos += 1;
                            text.push('\'');
                            continue;
                        }
                        break;
                    }
                    text.push(ch);
                }
            }
            _ => text.push('\''),
        }
    }

    fn argument(&mut self, in_plural: bool, depth: usize) -> Result<Part, ParseError> {
        self.skip_whitespace();
        let name = self.word()?;
        self.skip_whitespace();
        match self.bump() {
            Some('}') => return Ok(Part::Argument(name)),
            Some(',') => {}
            _ => return Err(self.error("expected `,` or `}` after argument name")),
        }
        self.skip_whitespace();
        let kind = self.word()?;
        self.skip_whitespace();
        match kind.as_str() {
            "number" => {
                self.expect('}')?;
                Ok(Part::Number(name))
            }
            "plural" => {
                self.expect(',')?;
                self.plural(name, depth)
            }
            "select" => {
                self.expect(',')?;
                self.select(name, in_plural, depth)
            }
            other => Err(self.error(&format!("unsupported argument type `{other}`"))),
        }
    }

    fn plural(&mut self, argument: String, depth: usize) -> Result<Part, ParseError> {
        let mut offset = 0;
        let mut cases = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => return Err(self.error("unterminated plural")),
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                _ => {}
            }
            if self.starts_with("offset:") {
                self.pos += "offset:".len();
                self.skip_whitespace();
                offset = self.integer()?;
                continue;
            }
            let key =
                if self.peek() == Some('=') {
                    self.pos += 1;
                    PluralKey::Exact(self.integer()?)
                } else {
                    let keyword = self.word()?;
                    PluralKey::Category(PluralCategory::parse(&keyword).ok_or_else(|| {
                        self.error(&format!("unknown plural category `{keyword}`"))
                    })?)
                };
            self.skip_whitespace();
            self.expect('{')?;
            let message = self.message(true, depth + 1)?;
            self.expect('}')?;
            cases.push((key, message));
        }
        if !cases
            .iter()
            .any(|(key, _)| *key == PluralKey::Category(PluralCategory::Other))
        {
            return Err(self.error(&format!("plural `{argument}` has no `other` case")));
        }
        Ok(Part::Plural {
            argument,
            offset,
            cases,
        })
    }

    fn select(
        &mut self,
        argument: String,
        in_plural: bool,
        depth: usize,
    ) -> Result<Part, ParseError> {
        let mut cases = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => return Err(self.error("unterminated select")),
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                _ => {}
            }
            let key = self.word()?;
            self.skip_whitespace();
            self.expect('{')?;
            let message = self.message(in_plural, depth + 1)?;
            self.expect('}')?;
            cases.push((key, message));
        }
        if !cases.iter().any(|(key, _)| key == "other") {
            return Err(self.error(&format!("select `{argument}` has no `other` case")));
        }
        Ok(Part::Select { argument, cases })
    }

    fn starts_with(&self, literal: &str) -> bool {
        literal
            .chars()
            .enumerate()
            .all(|(i, c)| self.chars.get(self.pos + i) == Some(&c))
    }
}

fn flush(text: &mut String, parts: &mut Vec<Part>) {
    if !text.is_empty() {
        parts.push(Part::Text(std::mem::take(text)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn de(source: &str, args: &Args) -> String {
        Message::parse(source)
            .unwrap()
            .render(Language::De, Locale::DeDe, args)
    }

    fn en(source: &str, args: &Args) -> String {
        Message::parse(source)
            .unwrap()
            .render(Language::En, Locale::EnUs, args)
    }

    #[test]
    fn plain_text_and_arguments() {
        assert_eq!(de("Hallo.", &Args::new()), "Hallo.");
        assert_eq!(
            en("Hello {name}!", &Args::new().str("name", "Ada")),
            "Hello Ada!"
        );
        assert_eq!(
            de(
                "{ name } und {other}",
                &Args::new().str("name", "A").str("other", "B")
            ),
            "A und B"
        );
    }

    #[test]
    fn numbers_follow_the_locale_not_the_string() {
        assert_eq!(
            de("{n} Zeilen", &Args::new().int("n", 1234)),
            "1.234 Zeilen"
        );
        assert_eq!(en("{n} rows", &Args::new().int("n", 1234)), "1,234 rows");
        assert_eq!(
            de("{x, number}", &Args::new().float("x", 1234.5)),
            "1.234,5"
        );
        assert_eq!(
            en("{x, number}", &Args::new().float("x", 1234.5)),
            "1,234.5"
        );
    }

    #[test]
    fn plural_picks_category_and_expands_pound() {
        let src = "{count, plural, one {# aktive Sprache} other {# aktive Sprachen}}";
        assert_eq!(de(src, &Args::new().int("count", 1)), "1 aktive Sprache");
        assert_eq!(de(src, &Args::new().int("count", 2)), "2 aktive Sprachen");
        assert_eq!(de(src, &Args::new().int("count", 0)), "0 aktive Sprachen");
        assert_eq!(
            de(src, &Args::new().int("count", 1200)),
            "1.200 aktive Sprachen"
        );
    }

    #[test]
    fn plural_exact_matches_win_and_offset_shifts_pound() {
        let src = "{n, plural, offset:1 =0 {nobody} =1 {only you} one {you and # other} other {you and # others}}";
        assert_eq!(en(src, &Args::new().int("n", 0)), "nobody");
        assert_eq!(en(src, &Args::new().int("n", 1)), "only you");
        assert_eq!(en(src, &Args::new().int("n", 2)), "you and 1 other");
        assert_eq!(en(src, &Args::new().int("n", 5)), "you and 4 others");
    }

    #[test]
    fn select_with_nested_argument_in_other() {
        let src = "Sprache: {language, select, de {Deutsch} en {English} other {{language}}}";
        assert_eq!(
            de(src, &Args::new().str("language", "de")),
            "Sprache: Deutsch"
        );
        assert_eq!(
            de(src, &Args::new().str("language", "en")),
            "Sprache: English"
        );
        assert_eq!(de(src, &Args::new().str("language", "fr")), "Sprache: fr");
    }

    #[test]
    fn pound_inside_select_inside_plural_still_means_the_count() {
        let src = "{n, plural, other {{g, select, f {# Frauen} other {# Personen}}}}";
        assert_eq!(de(src, &Args::new().int("n", 3).str("g", "f")), "3 Frauen");
    }

    #[test]
    fn apostrophe_quoting() {
        assert_eq!(de("it''s", &Args::new()), "it's");
        assert_eq!(
            de("'{'nicht ein Argument'}'", &Args::new()),
            "{nicht ein Argument}"
        );
        assert_eq!(de("'{name}'", &Args::new()), "{name}");
        assert_eq!(de("l'apostrophe", &Args::new()), "l'apostrophe");
        assert_eq!(
            de(
                "{n, plural, other {'#' ist Nummer #}}",
                &Args::new().int("n", 7)
            ),
            "# ist Nummer 7"
        );
        assert_eq!(
            de("Nr. # (kein Plural)", &Args::new()),
            "Nr. # (kein Plural)"
        );
    }

    #[test]
    fn missing_argument_stays_visible() {
        assert_eq!(de("Hallo {name}", &Args::new()), "Hallo {name}");
        assert_eq!(de("{n, plural, one {#} other {#}}", &Args::new()), "{n}");
    }

    #[test]
    fn arguments_are_collected_across_nesting() {
        let message = Message::parse(
            "{a} {b, number} {c, plural, other {{d}}} {e, select, x {{f}} other {}}",
        )
        .unwrap();
        let names: Vec<String> = message.arguments().into_iter().collect();
        assert_eq!(names, ["a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn rejects_malformed_messages() {
        let bad = [
            "unmatched }",
            "unmatched {name",
            "{count, plural, one {x}}",
            "{kind, select, a {x}}",
            "{count, plural, many-ish {x} other {y}}",
            "{x, date}",
            "{, plural, other {x}}",
        ];
        for source in bad {
            assert!(Message::parse(source).is_err(), "should reject: {source}");
        }
    }
}
