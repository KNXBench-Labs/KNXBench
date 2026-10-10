//! Bounded RTF views and supported-subset writing, never mutation of a source.
//!
//! RTF 1.9.1 Basic Entities/Unicode pp.7–16; explicit support is narrower than
//! that format. Call only for fields admitted as plain/RTF text. No IO, HTML,
//! object/image execution, plugin, resource lookup or KNX domain dependency.

use std::collections::BTreeSet;
use std::fmt;

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
pub const MAX_DEPTH: usize = 64;
pub const MAX_RUNS: usize = 4096;
const MAX_TOKENS: usize = 100_000;
const MAX_UNSUPPORTED: usize = 256;
const MAX_UNICODE_FALLBACK: usize = 16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub text: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    Plain,
    Rtf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInspection {
    pub format: TextFormat,
    pub runs: Vec<TextRun>,
    /// Distinct case-sensitive names, sorted; presence makes the preview partial.
    pub unsupported: Vec<String>,
}

impl TextInspection {
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextError {
    InputLimit,
    OutputLimit,
    DepthLimit,
    RunLimit,
    TokenLimit,
    UnsupportedLimit,
    Malformed,
    InvalidUnicode,
    UnsupportedEncoding,
    UnsupportedBinary,
    UnsupportedFormatting,
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InputLimit => "text source exceeds the preview budget",
            Self::OutputLimit => "text output exceeds the preview budget",
            Self::DepthLimit => "RTF nesting exceeds the preview budget",
            Self::RunLimit => "RTF style runs exceed the preview budget",
            Self::TokenLimit => "RTF tokens or Unicode fallback exceed the preview budget",
            Self::UnsupportedLimit => "RTF unsupported constructs exceed the preview budget",
            Self::Malformed => "malformed or unsupported RTF structure",
            Self::InvalidUnicode => "RTF Unicode code units are invalid or ambiguous",
            Self::UnsupportedEncoding => "RTF character encoding is unsupported or ambiguous",
            Self::UnsupportedBinary => "RTF binary payload preview is unsupported",
            Self::UnsupportedFormatting => "unsupported formatting prevents subset rewriting",
        })
    }
}
impl std::error::Error for TextError {}

#[derive(Debug, PartialEq)]
enum Token<'a> {
    Open,
    Close,
    Text(char),
    Symbol(char),
    Byte(u8),
    Word(&'a str, Option<i32>),
}
struct Cursor<'a> {
    input: &'a str,
    at: usize,
    tokens: usize,
}
impl<'a> Cursor<'a> {
    fn next(&mut self) -> Result<Option<Token<'a>>, TextError> {
        let bytes = self.input.as_bytes();
        if self.at == bytes.len() {
            return Ok(None);
        }
        self.tokens += 1;
        if self.tokens > MAX_TOKENS {
            return Err(TextError::TokenLimit);
        }
        let c = self.input[self.at..]
            .chars()
            .next()
            .ok_or(TextError::Malformed)?;
        self.at += c.len_utf8();
        Ok(Some(match c {
            '{' => Token::Open,
            '}' => Token::Close,
            '\\' => {
                let c = self.input[self.at..]
                    .chars()
                    .next()
                    .ok_or(TextError::Malformed)?;
                if c == '\'' {
                    self.at += 1;
                    let end = self.at.checked_add(2).ok_or(TextError::Malformed)?;
                    let hex = bytes.get(self.at..end).ok_or(TextError::Malformed)?;
                    if !hex.iter().all(u8::is_ascii_hexdigit) {
                        return Err(TextError::Malformed);
                    }
                    let value = u8::from_str_radix(&self.input[self.at..end], 16)
                        .map_err(|_| TextError::Malformed)?;
                    self.at = end;
                    Token::Byte(value)
                } else if c.is_ascii_alphabetic() {
                    let start = self.at;
                    while bytes.get(self.at).is_some_and(u8::is_ascii_alphabetic) {
                        self.at += 1;
                    }
                    if self.at - start > 32 {
                        return Err(TextError::Malformed);
                    }
                    let word = &self.input[start..self.at];
                    let number_start = self.at;
                    let minus = bytes.get(self.at) == Some(&b'-');
                    if minus {
                        self.at += 1;
                    }
                    let digits = self.at;
                    while bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                        self.at += 1;
                    }
                    let count = self.at - digits;
                    if count > 10 || (minus && count == 0) {
                        return Err(TextError::Malformed);
                    }
                    let number = if count == 0 {
                        None
                    } else {
                        Some(
                            self.input[number_start..self.at]
                                .parse::<i32>()
                                .map_err(|_| TextError::Malformed)?,
                        )
                    };
                    if bytes.get(self.at) == Some(&b' ') {
                        self.at += 1;
                    }
                    Token::Word(word, number)
                } else {
                    self.at += c.len_utf8();
                    Token::Symbol(c)
                }
            }
            c => Token::Text(c),
        }))
    }
}

#[derive(Clone, Copy)]
struct Frame {
    style: TextStyle,
    codepage: Option<i32>,
    unicode_fallback: usize,
    skip: bool,
    first: bool,
    ignorable: bool,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            style: TextStyle::default(),
            codepage: None,
            unicode_fallback: 1,
            skip: false,
            first: true,
            ignorable: false,
        }
    }
}
#[derive(Default)]
struct Output {
    runs: Vec<TextRun>,
    bytes: usize,
    surrogate: Option<(u16, TextStyle)>,
}
impl Output {
    fn character(&mut self, c: char, style: TextStyle) -> Result<(), TextError> {
        if self.surrogate.is_some() {
            return Err(TextError::InvalidUnicode);
        }
        self.bytes += c.len_utf8();
        if self.bytes > MAX_OUTPUT_BYTES {
            return Err(TextError::OutputLimit);
        }
        if let Some(run) = self.runs.last_mut().filter(|run| run.style == style) {
            run.text.push(c);
        } else {
            if self.runs.len() == MAX_RUNS {
                return Err(TextError::RunLimit);
            }
            self.runs.push(TextRun {
                text: c.to_string(),
                style,
            });
        }
        Ok(())
    }
    fn unicode(&mut self, value: i32, style: TextStyle) -> Result<(), TextError> {
        if !(-32768..=32767).contains(&value) {
            return Err(TextError::InvalidUnicode);
        }
        let unit = value as u16;
        if let Some((high, previous_style)) = self.surrogate.take() {
            if !(0xdc00..=0xdfff).contains(&unit) || style != previous_style {
                return Err(TextError::InvalidUnicode);
            }
            let c = char::decode_utf16([high, unit])
                .next()
                .ok_or(TextError::InvalidUnicode)?
                .map_err(|_| TextError::InvalidUnicode)?;
            return self.character(c, style);
        }
        if (0xd800..=0xdbff).contains(&unit) {
            self.surrogate = Some((unit, style));
            Ok(())
        } else {
            self.character(
                char::from_u32(u32::from(unit)).ok_or(TextError::InvalidUnicode)?,
                style,
            )
        }
    }
}

fn unsupported(names: &mut BTreeSet<String>, name: &str) -> Result<(), TextError> {
    names.insert(name.into());
    if names.len() > MAX_UNSUPPORTED {
        Err(TextError::UnsupportedLimit)
    } else {
        Ok(())
    }
}
fn destination(word: &str) -> bool {
    matches!(
        word,
        "fonttbl"
            | "colortbl"
            | "stylesheet"
            | "info"
            | "pict"
            | "object"
            | "field"
            | "filetbl"
            | "listtable"
            | "listoverridetable"
            | "header"
            | "footer"
            | "headerl"
            | "headerr"
            | "headerf"
            | "footerl"
            | "footerr"
            | "footerf"
            | "footnote"
            | "annotation"
            | "xmlnstbl"
            | "datastore"
            | "themedata"
            | "colorschememapping"
    )
}

// Windows-1252 special bytes; undefined codepoints deliberately refuse.
// Cross-checked with Python's standard cp1252 codec, not guessed replacement text.
const CP1252_HIGH: [Option<char>; 32] = [
    Some('\u{20ac}'),
    None,
    Some('\u{201a}'),
    Some('\u{192}'),
    Some('\u{201e}'),
    Some('\u{2026}'),
    Some('\u{2020}'),
    Some('\u{2021}'),
    Some('\u{2c6}'),
    Some('\u{2030}'),
    Some('\u{160}'),
    Some('\u{2039}'),
    Some('\u{152}'),
    None,
    Some('\u{17d}'),
    None,
    None,
    Some('\u{2018}'),
    Some('\u{2019}'),
    Some('\u{201c}'),
    Some('\u{201d}'),
    Some('\u{2022}'),
    Some('\u{2013}'),
    Some('\u{2014}'),
    Some('\u{2dc}'),
    Some('\u{2122}'),
    Some('\u{161}'),
    Some('\u{203a}'),
    Some('\u{153}'),
    None,
    Some('\u{17e}'),
    Some('\u{178}'),
];
fn byte_character(
    byte: u8,
    codepage: Option<i32>,
    ambiguous_font: bool,
) -> Result<char, TextError> {
    if byte < 128 {
        return Ok(char::from(byte));
    }
    if codepage != Some(1252) || ambiguous_font {
        return Err(TextError::UnsupportedEncoding);
    }
    if byte < 160 {
        CP1252_HIGH[usize::from(byte - 128)].ok_or(TextError::UnsupportedEncoding)
    } else {
        Ok(char::from(byte))
    }
}

/// Derive a bounded view; unknown formatting is partial, not rewrite authority.
/// A caller must keep the original independently and opt into RTF interpretation.
pub fn inspect_text(input: &str) -> Result<TextInspection, TextError> {
    if input.len() > MAX_SOURCE_BYTES {
        return Err(TextError::InputLimit);
    }
    if !input.starts_with("{\\rtf") {
        if input.len() > MAX_OUTPUT_BYTES {
            return Err(TextError::OutputLimit);
        }
        return Ok(TextInspection {
            format: TextFormat::Plain,
            runs: if input.is_empty() {
                Vec::new()
            } else {
                vec![TextRun {
                    text: input.into(),
                    style: TextStyle::default(),
                }]
            },
            unsupported: Vec::new(),
        });
    }
    let mut cursor = Cursor {
        input,
        at: 0,
        tokens: 0,
    };
    if cursor.next()? != Some(Token::Open) || cursor.next()? != Some(Token::Word("rtf", Some(1))) {
        return Err(TextError::Malformed);
    }
    let mut stack = vec![Frame {
        first: false,
        ..Frame::default()
    }];
    let mut out = Output::default();
    let mut names = BTreeSet::new();
    let mut fallback = 0;
    let mut ambiguous_font = false;
    while let Some(token) = cursor.next()? {
        match token {
            Token::Open => {
                fallback = 0;
                if stack.len() == MAX_DEPTH {
                    return Err(TextError::DepthLimit);
                }
                let parent = *stack.last().ok_or(TextError::Malformed)?;
                if parent.ignorable {
                    return Err(TextError::Malformed);
                }
                stack.push(Frame {
                    first: true,
                    ignorable: false,
                    ..parent
                });
                continue;
            }
            Token::Close => {
                fallback = 0;
                if stack.pop().ok_or(TextError::Malformed)?.ignorable {
                    return Err(TextError::Malformed);
                }
                if stack.is_empty() {
                    if !input[cursor.at..].chars().all(|c| c.is_ascii_whitespace()) {
                        return Err(TextError::Malformed);
                    }
                    if out.surrogate.is_some() {
                        return Err(TextError::InvalidUnicode);
                    }
                    return Ok(TextInspection {
                        format: TextFormat::Rtf,
                        runs: out.runs,
                        unsupported: names.into_iter().collect(),
                    });
                }
                continue;
            }
            Token::Word("bin", _) => return Err(TextError::UnsupportedBinary),
            Token::Text('\r' | '\n') => continue,
            _ => {}
        }
        let frame = stack.last_mut().ok_or(TextError::Malformed)?;
        if let Token::Text(c) = token {
            if !c.is_ascii() && !frame.skip {
                return Err(TextError::UnsupportedEncoding);
            }
        }
        if fallback > 0 {
            fallback -= 1;
            continue;
        }
        if frame.ignorable {
            if let Token::Word(word, _) = token {
                unsupported(&mut names, word)?;
                frame.skip = true;
                frame.ignorable = false;
                frame.first = false;
                continue;
            }
            return Err(TextError::Malformed);
        }
        if let Token::Symbol('*') = token {
            if !frame.first {
                return Err(TextError::Malformed);
            }
            frame.ignorable = true;
            continue;
        }
        if frame.skip {
            continue;
        }
        if let Token::Word(word, _) = token {
            if destination(word) {
                if !frame.first {
                    return Err(TextError::Malformed);
                }
                unsupported(&mut names, word)?;
                ambiguous_font |= word == "fonttbl";
                frame.skip = true;
                frame.first = false;
                continue;
            }
        }
        frame.first = false;
        match token {
            Token::Text(c) => out.character(c, frame.style)?,
            Token::Byte(byte) => out.character(
                byte_character(byte, frame.codepage, ambiguous_font)?,
                frame.style,
            )?,
            Token::Symbol('\\' | '{' | '}') => {
                if let Token::Symbol(c) = token {
                    out.character(c, frame.style)?;
                }
            }
            Token::Symbol('~') => out.character('\u{a0}', frame.style)?,
            Token::Symbol(c) => unsupported(&mut names, &format!("symbol:{c}"))?,
            Token::Word(word, number) => match word {
                "rtf" => return Err(TextError::Malformed),
                "ansi" | "pard" => {
                    if number.is_some() {
                        return Err(TextError::Malformed);
                    }
                }
                "ansicpg" => {
                    let number = number.ok_or(TextError::Malformed)?;
                    if number != 1252 {
                        return Err(TextError::UnsupportedEncoding);
                    }
                    frame.codepage = Some(number);
                }
                "uc" => {
                    let number = number.ok_or(TextError::Malformed)?;
                    if number < 0 {
                        return Err(TextError::Malformed);
                    }
                    let count = usize::try_from(number).map_err(|_| TextError::Malformed)?;
                    if count > MAX_UNICODE_FALLBACK {
                        return Err(TextError::TokenLimit);
                    }
                    frame.unicode_fallback = count;
                }
                "u" => {
                    out.unicode(number.ok_or(TextError::InvalidUnicode)?, frame.style)?;
                    fallback = frame.unicode_fallback;
                }
                "b" => frame.style.bold = number != Some(0),
                "i" => frame.style.italic = number != Some(0),
                "ul" => frame.style.underline = number != Some(0),
                "plain" | "ulnone" | "par" | "line" | "tab" => {
                    if number.is_some() {
                        return Err(TextError::Malformed);
                    }
                    match word {
                        "plain" => frame.style = TextStyle::default(),
                        "ulnone" => frame.style.underline = false,
                        "par" | "line" => out.character('\n', frame.style)?,
                        "tab" => out.character('\t', frame.style)?,
                        _ => unreachable!(),
                    }
                }
                _ => {
                    ambiguous_font |= matches!(word, "f" | "fcharset" | "cpg");
                    unsupported(&mut names, word)?;
                }
            },
            Token::Open | Token::Close => unreachable!(),
        }
    }
    Err(TextError::Malformed)
}

/// Encode text/basic run styles only. Original bytes are not regenerated;
/// callers must independently authorize conversion and preserve source content.
pub fn write_supported_rtf(view: &TextInspection) -> Result<String, TextError> {
    if !view.unsupported.is_empty() {
        return Err(TextError::UnsupportedFormatting);
    }
    if view.runs.len() > MAX_RUNS {
        return Err(TextError::RunLimit);
    }
    let mut bytes = 0;
    let mut result = String::from("{\\rtf1\\ansi\\ansicpg1252\\uc1 ");
    for run in &view.runs {
        bytes += run.text.len();
        if bytes > MAX_OUTPUT_BYTES {
            return Err(TextError::OutputLimit);
        }
        result.push_str("{\\plain");
        if run.style.bold {
            result.push_str("\\b");
        }
        if run.style.italic {
            result.push_str("\\i");
        }
        if run.style.underline {
            result.push_str("\\ul");
        }
        result.push(' ');
        for c in run.text.chars() {
            match c {
                '\\' | '{' | '}' => {
                    result.push('\\');
                    result.push(c);
                }
                '\n' => result.push_str("\\par "),
                '\t' => result.push_str("\\tab "),
                c if c.is_ascii() && !c.is_control() => result.push(c),
                c => {
                    for unit in c.encode_utf16(&mut [0; 2]) {
                        result.push_str(&format!("\\u{}?", *unit as i16));
                    }
                }
            }
            if result.len() > MAX_SOURCE_BYTES {
                return Err(TextError::InputLimit);
            }
        }
        result.push('}');
    }
    result.push('}');
    if result.len() > MAX_SOURCE_BYTES {
        return Err(TextError::InputLimit);
    }
    // Admission must be symmetric: a writer cannot produce an unpreviewable value.
    let written = inspect_text(&result)?;
    if written.plain_text() != view.plain_text() {
        return Err(TextError::InvalidUnicode);
    }
    Ok(result)
}
