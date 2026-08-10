use std::{
    fmt,
    fs::File,
    io::{self, BufRead, BufReader, Read},
    path::Path,
};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_saphyr::Spanned;

use crate::diagnostic::Diagnostic;

const MAX_FRONTMATTER_BYTES: usize = 8 * 1024 * 1024;

pub const SUPPORTED_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
    "when_to_use",
    "argument-hint",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "paths",
    "shell",
    "coordination",
    "skill-dependencies",
];

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Located<T> {
    pub value: T,
    pub line: u64,
    pub column: u64,
}

impl<T> Located<T> {
    fn at(value: T, location: serde_saphyr::Location) -> Self {
        Self { value, line: location.line(), column: location.column() }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FrontmatterField {
    pub name: String,
    pub line: u64,
    pub column: u64,
    #[serde(skip)]
    pub(crate) value: Located<FrontmatterValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FrontmatterValue {
    Null,
    Bool(bool),
    Integer(i128),
    String(String),
    Sequence(Vec<Located<FrontmatterValue>>),
    Mapping(Vec<FrontmatterMappingEntry>),
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontmatterMappingEntry {
    pub key: Located<String>,
    pub value: Located<FrontmatterValue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallTargets {
    ClaudeCode,
    Codex,
    ClaudeCodeAndCodex,
}

impl InstallTargets {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "claude-code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            "claude-code codex" => Some(Self::ClaudeCodeAndCodex),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "shape", content = "items", rename_all = "kebab-case")]
pub enum DependencyList {
    Sequence(Vec<Located<Option<String>>>),
    Invalid,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Frontmatter {
    pub fields: Vec<FrontmatterField>,
    pub name: Option<Located<String>>,
    pub description: Option<Located<String>>,
    pub compatibility: Option<Located<String>>,
    pub argument_hint: Option<Located<String>>,
    pub user_invocable: Option<Located<bool>>,
    pub disable_model_invocation: Option<Located<bool>>,
    pub model: Option<Located<String>>,
    pub effort: Option<Located<String>>,
    pub context: Option<Located<String>>,
    pub agent: Option<Located<String>>,
    pub background: Option<Located<bool>>,
    pub shell: Option<Located<String>>,
    pub install_targets: Option<Located<Option<InstallTargets>>>,
    pub coordination: Option<Located<String>>,
    pub skill_dependencies: Option<Located<DependencyList>>,
}

impl Frontmatter {
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.iter().any(|field| field.name == name)
    }

    pub fn field(&self, name: &str) -> Option<&FrontmatterField> {
        self.fields.iter().find(|field| field.name == name)
    }

    pub(crate) fn value(&self, name: &str) -> Option<&Located<FrontmatterValue>> {
        self.field(name).map(|field| &field.value)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrontmatterParse {
    pub frontmatter: Option<Frontmatter>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_skill_file(path: &Path) -> FrontmatterParse {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return FrontmatterParse {
                frontmatter: None,
                diagnostics: vec![Diagnostic::error(
                    "FRONTMATTER_READ_ERROR",
                    path,
                    1,
                    1,
                    format!("could not read SKILL.md: {error}"),
                )],
            };
        }
    };
    parse_reader(path, BufReader::new(file))
}

fn parse_reader(path: &Path, mut reader: impl BufRead) -> FrontmatterParse {
    let mut consumed = 0usize;
    let mut line_number = 0u64;
    let mut line = Vec::new();
    match read_bounded_line(&mut reader, &mut line, &mut consumed) {
        Ok(ReadLine::Complete(0)) => {
            return missing_delimiter(path, 1, "missing opening YAML frontmatter delimiter");
        }
        Ok(ReadLine::Complete(_)) => {}
        Ok(ReadLine::LimitExceeded) => return too_large(path, 1),
        Err(error) => return read_error(path, 1, error),
    }
    line_number += 1;
    if delimiter_contents(&line) != Some(b"---") {
        return missing_delimiter(path, 1, "missing opening YAML frontmatter delimiter");
    }

    // Preserve the physical source line numbers after removing the opening delimiter.
    let mut yaml = Vec::with_capacity(4096);
    yaml.push(b'\n');
    loop {
        line.clear();
        match read_bounded_line(&mut reader, &mut line, &mut consumed) {
            Ok(ReadLine::Complete(0)) => {
                return missing_delimiter(
                    path,
                    line_number.saturating_add(1),
                    "missing closing YAML frontmatter delimiter",
                );
            }
            Ok(ReadLine::Complete(_)) => {}
            Ok(ReadLine::LimitExceeded) => return too_large(path, line_number.saturating_add(1)),
            Err(error) => return read_error(path, line_number.saturating_add(1), error),
        }
        line_number += 1;
        if delimiter_contents(&line) == Some(b"---") {
            break;
        }
        yaml.extend_from_slice(&line);
    }

    let yaml = match String::from_utf8(yaml) {
        Ok(yaml) => yaml,
        Err(error) => {
            let line =
                1 + error.as_bytes()[..error.utf8_error().valid_up_to()].iter().filter(|byte| **byte == b'\n').count()
                    as u64;
            return FrontmatterParse {
                frontmatter: None,
                diagnostics: vec![Diagnostic::error(
                    "FRONTMATTER_INVALID_UTF8",
                    path,
                    line,
                    1,
                    "YAML frontmatter is not valid UTF-8",
                )],
            };
        }
    };

    let document: Spanned<YamlValue> = match serde_saphyr::from_str(&yaml) {
        Ok(document) => document,
        Err(error) => {
            let location = error.location().unwrap_or(serde_saphyr::Location::UNKNOWN);
            return FrontmatterParse {
                frontmatter: None,
                diagnostics: vec![Diagnostic::error(
                    "FRONTMATTER_INVALID_YAML",
                    path,
                    location.line().max(2),
                    location.column().max(1),
                    format!("invalid YAML frontmatter: {error}"),
                )],
            };
        }
    };

    let YamlValue::Mapping(fields) = document.value else {
        return FrontmatterParse {
            frontmatter: None,
            diagnostics: vec![Diagnostic::error(
                "FRONTMATTER_NOT_MAPPING",
                path,
                document.referenced.line().max(2),
                document.referenced.column().max(1),
                "YAML frontmatter must be a mapping",
            )],
        };
    };

    FrontmatterParse { frontmatter: Some(extract_frontmatter(&fields)), diagnostics: Vec::new() }
}

fn extract_frontmatter(fields: &[(Spanned<String>, Spanned<YamlValue>)]) -> Frontmatter {
    let mut frontmatter = Frontmatter {
        fields: fields
            .iter()
            .map(|(key, value)| FrontmatterField {
                name: key.value.clone(),
                line: key.referenced.line(),
                column: key.referenced.column(),
                value: Located::at(frontmatter_value(&value.value), value.referenced),
            })
            .collect(),
        ..Frontmatter::default()
    };

    for (key, value) in fields {
        let field_location = key.referenced;
        match key.value.as_str() {
            "name" => frontmatter.name = string_value(value, field_location),
            "description" => frontmatter.description = string_value(value, field_location),
            "compatibility" => frontmatter.compatibility = string_value(value, field_location),
            "argument-hint" => frontmatter.argument_hint = string_value(value, field_location),
            "user-invocable" => frontmatter.user_invocable = bool_value(value, field_location),
            "disable-model-invocation" => {
                frontmatter.disable_model_invocation = bool_value(value, field_location);
            }
            "model" => frontmatter.model = string_value(value, field_location),
            "effort" => frontmatter.effort = string_value(value, field_location),
            "context" => frontmatter.context = string_value(value, field_location),
            "agent" => frontmatter.agent = string_value(value, field_location),
            "background" => frontmatter.background = bool_value(value, field_location),
            "shell" => frontmatter.shell = string_value(value, field_location),
            "coordination" => frontmatter.coordination = string_value(value, field_location),
            "metadata" => {
                if let YamlValue::Mapping(metadata) = &value.value &&
                    let Some((key, value)) = metadata.iter().find(|(key, _)| key.value == "install-targets")
                {
                    let parsed = match &value.value {
                        YamlValue::String(value) => InstallTargets::parse(value),
                        _ => None,
                    };
                    frontmatter.install_targets = Some(Located::at(parsed, key.referenced));
                }
            }
            "skill-dependencies" => {
                let dependency_list = match &value.value {
                    YamlValue::Sequence(items) => DependencyList::Sequence(
                        items
                            .iter()
                            .map(|item| {
                                Located::at(
                                    match &item.value {
                                        YamlValue::String(value) => Some(value.clone()),
                                        _ => None,
                                    },
                                    item.referenced,
                                )
                            })
                            .collect(),
                    ),
                    _ => DependencyList::Invalid,
                };
                frontmatter.skill_dependencies = Some(Located::at(dependency_list, field_location));
            }
            "license" | "allowed-tools" | "when_to_use" | "arguments" | "disallowed-tools" | "hooks" | "paths" => {}
            _ => {}
        }
    }
    frontmatter
}

fn string_value(value: &Spanned<YamlValue>, location: serde_saphyr::Location) -> Option<Located<String>> {
    match &value.value {
        YamlValue::String(value) => Some(Located::at(value.clone(), location)),
        _ => None,
    }
}

fn bool_value(value: &Spanned<YamlValue>, location: serde_saphyr::Location) -> Option<Located<bool>> {
    let parsed = match &value.value {
        YamlValue::Bool(value) => Some(*value),
        YamlValue::Integer(0) => Some(false),
        YamlValue::Integer(1) => Some(true),
        YamlValue::String(value)
            if value.eq_ignore_ascii_case("false") ||
                value.eq_ignore_ascii_case("no") ||
                value.eq_ignore_ascii_case("off") =>
        {
            Some(false)
        }
        YamlValue::String(value)
            if value.eq_ignore_ascii_case("true") ||
                value.eq_ignore_ascii_case("yes") ||
                value.eq_ignore_ascii_case("on") =>
        {
            Some(true)
        }
        _ => None,
    };
    parsed.map(|value| Located::at(value, location))
}

fn frontmatter_value(value: &YamlValue) -> FrontmatterValue {
    match value {
        YamlValue::Null => FrontmatterValue::Null,
        YamlValue::Bool(value) => FrontmatterValue::Bool(*value),
        YamlValue::Integer(value) => FrontmatterValue::Integer(*value),
        YamlValue::String(value) => FrontmatterValue::String(value.clone()),
        YamlValue::Sequence(values) => FrontmatterValue::Sequence(
            values.iter().map(|value| Located::at(frontmatter_value(&value.value), value.referenced)).collect(),
        ),
        YamlValue::Mapping(values) => FrontmatterValue::Mapping(
            values
                .iter()
                .map(|(key, value)| FrontmatterMappingEntry {
                    key: Located::at(key.value.clone(), key.referenced),
                    value: Located::at(frontmatter_value(&value.value), value.referenced),
                })
                .collect(),
        ),
        YamlValue::Other => FrontmatterValue::Other,
    }
}

pub(crate) fn markdown_prose(markdown: &str) -> String {
    let mut visible = markdown.as_bytes().to_vec();
    let mut offset = 0usize;
    let mut fence = None;
    let mut example_level = None;

    for line in markdown.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        let indent = content.bytes().take_while(|byte| *byte == b' ').count();
        let trimmed = &content[indent.min(content.len())..];
        let marker = (indent <= 3).then(|| fence_marker(trimmed)).flatten();

        if let Some((kind, width)) = fence {
            mask(&mut visible, offset, offset + content.len());
            if marker.is_some_and(|(candidate, candidate_width)| candidate == kind && candidate_width >= width) {
                fence = None;
            }
        } else if let Some(marker) = marker {
            fence = Some(marker);
            mask(&mut visible, offset, offset + content.len());
        } else {
            let heading = markdown_heading(trimmed);
            if let Some((level, _)) = heading &&
                example_level.is_some_and(|example| level <= example)
            {
                example_level = None;
            }
            if let Some((level, title)) = heading &&
                is_example_heading(title)
            {
                example_level = Some(level);
            }

            if example_level.is_some() || indent >= 4 || trimmed.starts_with('>') {
                mask(&mut visible, offset, offset + content.len());
            } else {
                mask_inline_code(content.as_bytes(), &mut visible, offset);
            }
        }
        offset += line.len();
    }

    String::from_utf8(visible).expect("masking Markdown preserves UTF-8")
}

fn fence_marker(line: &str) -> Option<(u8, usize)> {
    let kind = *line.as_bytes().first()?;
    if !matches!(kind, b'`' | b'~') {
        return None;
    }
    let width = line.bytes().take_while(|byte| *byte == kind).count();
    (width >= 3).then_some((kind, width))
}

fn markdown_heading(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&level) || line.as_bytes().get(level) != Some(&b' ') {
        return None;
    }
    Some((level, line[level + 1..].trim().trim_end_matches('#').trim_end()))
}

fn is_example_heading(title: &str) -> bool {
    let title = title.to_ascii_lowercase();
    matches!(title.as_str(), "example" | "examples") || title.starts_with("example:") || title.starts_with("examples:")
}

fn mask_inline_code(line: &[u8], visible: &mut [u8], offset: usize) {
    let mut start = 0usize;
    while let Some(relative) = line[start..].iter().position(|byte| *byte == b'`') {
        let opening = start + relative;
        let width = line[opening..].iter().take_while(|byte| **byte == b'`').count();
        let mut search = opening + width;
        let closing = loop {
            let Some(relative) = line[search..].iter().position(|byte| *byte == b'`') else {
                break None;
            };
            let candidate = search + relative;
            let candidate_width = line[candidate..].iter().take_while(|byte| **byte == b'`').count();
            if candidate_width == width {
                break Some(candidate);
            }
            search = candidate + candidate_width;
        };
        let Some(closing) = closing else {
            break;
        };
        mask(visible, offset + opening, offset + closing + width);
        start = closing + width;
    }
}

fn mask(bytes: &mut [u8], start: usize, end: usize) {
    bytes[start..end].fill(b' ');
}

enum ReadLine {
    Complete(usize),
    LimitExceeded,
}

fn read_bounded_line(reader: &mut impl BufRead, line: &mut Vec<u8>, consumed: &mut usize) -> io::Result<ReadLine> {
    let remaining = MAX_FRONTMATTER_BYTES.saturating_sub(*consumed);
    if remaining == 0 {
        return Ok(ReadLine::LimitExceeded);
    }
    let read = reader.take((remaining + 1) as u64).read_until(b'\n', line)?;
    *consumed = consumed.saturating_add(read);
    if *consumed > MAX_FRONTMATTER_BYTES { Ok(ReadLine::LimitExceeded) } else { Ok(ReadLine::Complete(read)) }
}

fn delimiter_contents(line: &[u8]) -> Option<&[u8]> {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let line = line.trim_ascii_end();
    Some(line).filter(|line| *line == b"---")
}

fn missing_delimiter(path: &Path, line: u64, message: &'static str) -> FrontmatterParse {
    FrontmatterParse {
        frontmatter: None,
        diagnostics: vec![Diagnostic::error("FRONTMATTER_DELIMITER_MISSING", path, line, 1, message)],
    }
}

fn too_large(path: &Path, line: u64) -> FrontmatterParse {
    FrontmatterParse {
        frontmatter: None,
        diagnostics: vec![Diagnostic::error(
            "FRONTMATTER_TOO_LARGE",
            path,
            line,
            1,
            format!("YAML frontmatter exceeds {MAX_FRONTMATTER_BYTES} bytes"),
        )],
    }
}

fn read_error(path: &Path, line: u64, error: io::Error) -> FrontmatterParse {
    FrontmatterParse {
        frontmatter: None,
        diagnostics: vec![Diagnostic::error(
            "FRONTMATTER_READ_ERROR",
            path,
            line,
            1,
            format!("could not read SKILL.md: {error}"),
        )],
    }
}

#[derive(Clone, Debug, PartialEq)]
enum YamlValue {
    Null,
    Bool(bool),
    Integer(i128),
    String(String),
    Sequence(Vec<Spanned<YamlValue>>),
    Mapping(Vec<(Spanned<String>, Spanned<YamlValue>)>),
    Other,
}

impl<'de> Deserialize<'de> for YamlValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(YamlValueVisitor)
    }
}

struct YamlValueVisitor;

impl<'de> Visitor<'de> for YamlValueVisitor {
    type Value = YamlValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(YamlValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(YamlValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(YamlValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(YamlValue::Integer(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(YamlValue::Integer(value.into()))
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(YamlValue::Other)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(YamlValue::String(value.to_owned()))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(YamlValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(YamlValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(1024));
        while let Some(value) = sequence.next_element::<Spanned<YamlValue>>()? {
            values.push(value);
        }
        Ok(YamlValue::Sequence(values))
    }

    fn visit_map<A>(self, mut mapping: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Vec::with_capacity(mapping.size_hint().unwrap_or(0).min(1024));
        while let Some(key) = mapping.next_key::<Spanned<String>>()? {
            let value = mapping.next_value::<Spanned<YamlValue>>()?;
            values.push((key, value));
        }
        Ok(YamlValue::Mapping(values))
    }
}
