//! A small JSONC parser whose edits leave all untouched source verbatim.

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum JsoncError {
    #[error("JSONC parse error at byte {position}: {message}")]
    Parse { position: usize, message: String },
}

#[derive(Clone, Debug)]
pub(crate) struct ValueNode {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: NodeKind,
}

#[derive(Clone, Debug)]
pub(crate) enum NodeKind {
    Scalar(Value),
    Object(ObjectNode),
    Array(ArrayNode),
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectMember {
    pub(crate) key: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) value: ValueNode,
    pub(crate) comma: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectNode {
    pub(crate) opening: usize,
    pub(crate) closing: usize,
    pub(crate) members: Vec<ObjectMember>,
}

#[derive(Clone, Debug)]
pub(crate) struct ArrayElement {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) value: ValueNode,
    pub(crate) comma: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct ArrayNode {
    pub(crate) opening: usize,
    pub(crate) closing: usize,
    pub(crate) elements: Vec<ArrayElement>,
}

#[derive(Clone, Debug)]
pub(crate) struct JsoncDocument {
    pub(crate) text: String,
    pub(crate) root: ValueNode,
}

impl ValueNode {
    pub(crate) fn value(&self) -> Value {
        match &self.kind {
            NodeKind::Scalar(value) => value.clone(),
            NodeKind::Object(object) => {
                Value::Object(object.members.iter().map(|member| (member.key.clone(), member.value.value())).collect())
            }
            NodeKind::Array(array) => {
                Value::Array(array.elements.iter().map(|element| element.value.value()).collect())
            }
        }
    }

    pub(crate) fn object(&self) -> Option<&ObjectNode> {
        match &self.kind {
            NodeKind::Object(object) => Some(object),
            _ => None,
        }
    }

    pub(crate) fn array(&self) -> Option<&ArrayNode> {
        match &self.kind {
            NodeKind::Array(array) => Some(array),
            _ => None,
        }
    }
}

impl JsoncDocument {
    pub(crate) fn parse(text: impl Into<String>) -> Result<Self, JsoncError> {
        let text = text.into();
        let mut parser = Parser { text: &text, index: 0 };
        let root = parser.parse_value()?;
        parser.skip_trivia()?;
        if parser.index != text.len() {
            return Err(parser.error("extra data"));
        }
        Ok(Self { text, root })
    }

    pub(crate) fn value(&self) -> Value {
        self.root.value()
    }

    pub(crate) fn member<'a>(&self, object: &'a ObjectNode, key: &str) -> Option<&'a ObjectMember> {
        object.members.iter().find(|member| member.key == key)
    }

    pub(crate) fn replace_value(&self, node: &ValueNode, value: &Value) -> Result<Self, JsoncError> {
        self.replace(node.start, node.end, &render_value(value, &indent_at(&self.text, node.start)))
    }

    pub(crate) fn insert_member(&self, object: &ObjectNode, key: &str, value: &Value) -> Result<Self, JsoncError> {
        let indentation = container_indentation(&self.text, object.opening, object.closing);
        let child_indentation = format!("{indentation}  ");
        let member = format!(
            "{}: {}",
            serde_json::to_string(key).expect("string serializes"),
            render_value(value, &child_indentation)
        );
        if let Some(last) = object.members.last() {
            let document =
                if last.comma.is_none() { self.replace(last.value.end, last.value.end, ",")? } else { self.clone() };
            let object = find_object(&document.root, object.opening).expect("object survives insertion");
            let suffix = if last.comma.is_some() { "," } else { "" };
            document.insert_before_closing(object, &format!("{member}{suffix}"))
        } else {
            self.insert_before_closing(object, &member)
        }
    }

    pub(crate) fn append_element(&self, array: &ArrayNode, value: &Value) -> Result<Self, JsoncError> {
        let indentation = container_indentation(&self.text, array.opening, array.closing);
        let child_indentation = format!("{indentation}  ");
        let element = render_value(value, &child_indentation);
        if let Some(last) = array.elements.last() {
            let document =
                if last.comma.is_none() { self.replace(last.value.end, last.value.end, ",")? } else { self.clone() };
            let array = find_array(&document.root, array.opening).expect("array survives insertion");
            let suffix = if last.comma.is_some() { "," } else { "" };
            document.insert_before_closing_array(array, &format!("{element}{suffix}"))
        } else {
            self.insert_before_closing_array(array, &element)
        }
    }

    pub(crate) fn remove_member(&self, object: &ObjectNode, index: usize) -> Result<Self, JsoncError> {
        let member = &object.members[index];
        if let Some(comma) = member.comma {
            self.replace(member.start, comma + 1, "")
        } else if index > 0 {
            let previous = &object.members[index - 1];
            self.replace(previous.comma.expect("preceding member has comma"), member.end, "")
        } else {
            self.replace(member.start, member.end, "")
        }
    }

    pub(crate) fn remove_element(&self, array: &ArrayNode, index: usize) -> Result<Self, JsoncError> {
        let element = &array.elements[index];
        if let Some(comma) = element.comma {
            self.replace(element.start, comma + 1, "")
        } else if index > 0 {
            let previous = &array.elements[index - 1];
            self.replace(previous.comma.expect("preceding element has comma"), element.end, "")
        } else {
            self.replace(element.start, element.end, "")
        }
    }

    fn insert_before_closing(&self, object: &ObjectNode, rendered: &str) -> Result<Self, JsoncError> {
        self.insert_before(object.opening, object.closing, rendered)
    }

    fn insert_before_closing_array(&self, array: &ArrayNode, rendered: &str) -> Result<Self, JsoncError> {
        self.insert_before(array.opening, array.closing, rendered)
    }

    fn insert_before(&self, opening: usize, closing: usize, rendered: &str) -> Result<Self, JsoncError> {
        let indentation = container_indentation(&self.text, opening, closing);
        let before_closing = &self.text[opening + 1..closing];
        let prefix = if before_closing.contains('\n') || before_closing.contains('\r') {
            "  ".to_owned()
        } else {
            format!("\n{indentation}  ")
        };
        self.replace(closing, closing, &format!("{prefix}{rendered}\n{indentation}"))
    }

    fn replace(&self, start: usize, end: usize, replacement: &str) -> Result<Self, JsoncError> {
        let mut text = String::with_capacity(self.text.len() + replacement.len() - (end - start));
        text.push_str(&self.text[..start]);
        text.push_str(replacement);
        text.push_str(&self.text[end..]);
        Self::parse(text)
    }
}

struct Parser<'a> {
    text: &'a str,
    index: usize,
}

impl Parser<'_> {
    fn parse_value(&mut self) -> Result<ValueNode, JsoncError> {
        self.skip_trivia()?;
        let start = self.index;
        let Some(character) = self.peek() else {
            return Err(self.error("expecting value"));
        };
        match character {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '"' => {
                let value = Value::String(self.parse_string()?);
                Ok(ValueNode { start, end: self.index, kind: NodeKind::Scalar(value) })
            }
            _ => {
                while let Some(character) = self.peek() {
                    if character.is_whitespace() ||
                        matches!(character, ',' | ']' | '}') ||
                        self.text[self.index..].starts_with("//") ||
                        self.text[self.index..].starts_with("/*")
                    {
                        break;
                    }
                    self.bump();
                }
                if self.index == start {
                    return Err(self.error("expecting value"));
                }
                let raw = &self.text[start..self.index];
                let value = serde_json::from_str(raw).map_err(|error| JsoncError::Parse {
                    position: start + error.column().saturating_sub(1),
                    message: error.to_string(),
                })?;
                Ok(ValueNode { start, end: self.index, kind: NodeKind::Scalar(value) })
            }
        }
    }

    fn parse_object(&mut self) -> Result<ValueNode, JsoncError> {
        let start = self.index;
        self.bump();
        let opening = start;
        let mut members = Vec::new();
        self.skip_trivia()?;
        while self.peek() != Some('}') {
            let member_start = self.index;
            if self.peek() != Some('"') {
                return Err(self.error("expecting property name"));
            }
            let key = self.parse_string()?;
            self.skip_trivia()?;
            if self.peek() != Some(':') {
                return Err(self.error("expecting ':' delimiter"));
            }
            self.bump();
            let value = self.parse_value()?;
            self.skip_trivia()?;
            let comma = if self.peek() == Some(',') {
                let at = self.index;
                self.bump();
                self.skip_trivia()?;
                Some(at)
            } else {
                None
            };
            let end = value.end;
            members.push(ObjectMember { key, start: member_start, end, value, comma });
            if comma.is_none() {
                break;
            }
        }
        if self.peek() != Some('}') {
            return Err(self.error("expecting ',' delimiter"));
        }
        let closing = self.index;
        self.bump();
        Ok(ValueNode { start, end: self.index, kind: NodeKind::Object(ObjectNode { opening, closing, members }) })
    }

    fn parse_array(&mut self) -> Result<ValueNode, JsoncError> {
        let start = self.index;
        self.bump();
        let opening = start;
        let mut elements = Vec::new();
        self.skip_trivia()?;
        while self.peek() != Some(']') {
            let value = self.parse_value()?;
            self.skip_trivia()?;
            let comma = if self.peek() == Some(',') {
                let at = self.index;
                self.bump();
                self.skip_trivia()?;
                Some(at)
            } else {
                None
            };
            elements.push(ArrayElement { start: value.start, end: value.end, value, comma });
            if comma.is_none() {
                break;
            }
        }
        if self.peek() != Some(']') {
            return Err(self.error("expecting ',' delimiter"));
        }
        let closing = self.index;
        self.bump();
        Ok(ValueNode { start, end: self.index, kind: NodeKind::Array(ArrayNode { opening, closing, elements }) })
    }

    fn parse_string(&mut self) -> Result<String, JsoncError> {
        let start = self.index;
        self.bump();
        let mut escaped = false;
        while let Some(character) = self.peek() {
            if escaped {
                escaped = false;
                self.bump();
                continue;
            }
            match character {
                '\\' => {
                    escaped = true;
                    self.bump();
                }
                '"' => {
                    self.bump();
                    return serde_json::from_str(&self.text[start..self.index]).map_err(|error| JsoncError::Parse {
                        position: start + error.column().saturating_sub(1),
                        message: error.to_string(),
                    });
                }
                '\n' | '\r' => return Err(self.error("invalid control character")),
                _ => self.bump(),
            }
        }
        Err(JsoncError::Parse { position: start, message: "unterminated string".to_owned() })
    }

    fn skip_trivia(&mut self) -> Result<(), JsoncError> {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.bump();
            }
            if self.text[self.index..].starts_with("//") {
                self.index += 2;
                while self.peek().is_some_and(|character| character != '\n' && character != '\r') {
                    self.bump();
                }
            } else if self.text[self.index..].starts_with("/*") {
                let start = self.index;
                self.index += 2;
                if let Some(end) = self.text[self.index..].find("*/") {
                    self.index += end + 2;
                } else {
                    return Err(JsoncError::Parse {
                        position: start,
                        message: "unterminated block comment".to_owned(),
                    });
                }
            } else {
                return Ok(());
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.text[self.index..].chars().next()
    }
    fn bump(&mut self) {
        self.index += self.peek().expect("only called at a character boundary").len_utf8();
    }
    fn error(&self, message: &str) -> JsoncError {
        JsoncError::Parse { position: self.index, message: message.to_owned() }
    }
}

fn find_object(node: &ValueNode, opening: usize) -> Option<&ObjectNode> {
    match &node.kind {
        NodeKind::Object(object) if object.opening == opening => Some(object),
        NodeKind::Object(object) => object.members.iter().find_map(|member| find_object(&member.value, opening)),
        NodeKind::Array(array) => array.elements.iter().find_map(|element| find_object(&element.value, opening)),
        NodeKind::Scalar(_) => None,
    }
}

fn find_array(node: &ValueNode, opening: usize) -> Option<&ArrayNode> {
    match &node.kind {
        NodeKind::Object(object) => object.members.iter().find_map(|member| find_array(&member.value, opening)),
        NodeKind::Array(array) if array.opening == opening => Some(array),
        NodeKind::Array(array) => array.elements.iter().find_map(|element| find_array(&element.value, opening)),
        NodeKind::Scalar(_) => None,
    }
}

fn indent_at(text: &str, position: usize) -> String {
    let line_start = text[..position].rfind('\n').map_or(0, |index| index + 1);
    let indentation = &text[line_start..position];
    if indentation.chars().all(char::is_whitespace) { indentation.to_owned() } else { String::new() }
}

fn container_indentation(text: &str, opening: usize, closing: usize) -> String {
    let indentation = indent_at(text, closing);
    if !indentation.is_empty() {
        return indentation;
    }
    let line_start = text[..opening].rfind('\n').map_or(0, |index| index + 1);
    text[line_start..opening].chars().take_while(|character| character.is_whitespace()).collect()
}

fn render_value(value: &Value, indentation: &str) -> String {
    let rendered = serde_json::to_string_pretty(value).expect("JSON value serializes");
    let mut lines = rendered.lines();
    let Some(first) = lines.next() else {
        return rendered;
    };
    std::iter::once(first.to_owned())
        .chain(lines.map(|line| format!("{indentation}{line}")))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::JsoncDocument;

    #[test]
    fn edits_jsonc_without_reformatting_unrelated_content() {
        let document = JsoncDocument::parse("{\n  // preserved\n  \"other\": [1,],\n}").unwrap();
        let root = document.root.object().unwrap();
        let document = document.insert_member(root, "hooks", &json!({})).unwrap();

        assert_eq!(document.text, "{\n  // preserved\n  \"other\": [1,],\n  \"hooks\": {},\n}");
    }

    #[test]
    fn parses_unicode_comments_and_trailing_commas() {
        let document = JsoncDocument::parse("{/* α */\"x\": [\"✓\",],}").unwrap();
        assert_eq!(document.value(), json!({"x": ["✓"]}));
    }

    #[test]
    fn parses_comments_adjacent_to_literals() {
        let document = JsoncDocument::parse("{\"enabled\":true/* intentional */}").unwrap();
        assert_eq!(document.value(), json!({"enabled": true}));
    }
}
