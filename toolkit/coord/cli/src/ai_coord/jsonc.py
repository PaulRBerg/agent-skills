"""Minimal JSONC parser and source-preserving editor."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, NoReturn


@dataclass(frozen=True, slots=True)
class ValueNode:
    start: int
    end: int
    value: Any


@dataclass(frozen=True, slots=True)
class ObjectMember:
    key: str
    start: int
    end: int
    value: ValueNode
    comma: int | None


@dataclass(frozen=True, slots=True)
class ObjectNode(ValueNode):
    opening: int
    closing: int
    members: tuple[ObjectMember, ...]


@dataclass(frozen=True, slots=True)
class ArrayElement:
    start: int
    end: int
    value: ValueNode
    comma: int | None


@dataclass(frozen=True, slots=True)
class ArrayNode(ValueNode):
    opening: int
    closing: int
    elements: tuple[ArrayElement, ...]


class JsoncDocument:
    """A parsed JSONC document whose unchanged source is emitted verbatim."""

    def __init__(self, text: str, root: ValueNode) -> None:
        self.text = text
        self.root = root

    @classmethod
    def parse(cls, text: str) -> JsoncDocument:
        parser = _Parser(text)
        root = parser.parse_value()
        parser.skip_trivia()
        if parser.index != len(text):
            parser.error("Extra data")
        return cls(text, root)

    @property
    def value(self) -> Any:
        return self.root.value

    def member(self, object_node: ObjectNode, key: str) -> ObjectMember | None:
        return next((member for member in object_node.members if member.key == key), None)

    def replace_value(self, node: ValueNode, value: Any) -> JsoncDocument:
        return self._replace(
            node.start, node.end, _render_value(value, _indent_at(self.text, node.start))
        )

    def insert_member(self, object_node: ObjectNode, key: str, value: Any) -> JsoncDocument:
        indentation = _container_indentation(self.text, object_node)
        child_indentation = f"{indentation}  "
        member = f"{json.dumps(key)}: {_render_value(value, child_indentation)}"
        if object_node.members:
            last = object_node.members[-1]
            document = self
            trailing_comma = last.comma is not None
            if last.comma is None:
                document = document._replace(last.value.end, last.value.end, ",")
                updated_object = _object_at(document.root, object_node.start)
                assert updated_object is not None
                object_node = updated_object
            suffix = "," if trailing_comma else ""
            return document._insert_before_closing(object_node, f"{member}{suffix}")
        return self._insert_before_closing(object_node, member)

    def append_element(self, array_node: ArrayNode, value: Any) -> JsoncDocument:
        indentation = _container_indentation(self.text, array_node)
        child_indentation = f"{indentation}  "
        element = _render_value(value, child_indentation)
        if array_node.elements:
            last = array_node.elements[-1]
            document = self
            trailing_comma = last.comma is not None
            if last.comma is None:
                document = document._replace(last.value.end, last.value.end, ",")
                updated_array = _array_at(document.root, array_node.start)
                assert updated_array is not None
                array_node = updated_array
            suffix = "," if trailing_comma else ""
            return document._insert_before_closing(array_node, f"{element}{suffix}")
        return self._insert_before_closing(array_node, element)

    def remove_member(self, object_node: ObjectNode, index: int) -> JsoncDocument:
        member = object_node.members[index]
        if member.comma is not None:
            return self._replace(member.start, member.comma + 1, "")
        if index:
            previous = object_node.members[index - 1]
            assert previous.comma is not None
            return self._replace(previous.comma, member.end, "")
        return self._replace(member.start, member.end, "")

    def remove_element(self, array_node: ArrayNode, index: int) -> JsoncDocument:
        element = array_node.elements[index]
        if element.comma is not None:
            return self._replace(element.start, element.comma + 1, "")
        if index:
            previous = array_node.elements[index - 1]
            assert previous.comma is not None
            return self._replace(previous.comma, element.end, "")
        return self._replace(element.start, element.end, "")

    def _insert_before_closing(self, node: ObjectNode | ArrayNode, rendered: str) -> JsoncDocument:
        indentation = _container_indentation(self.text, node)
        before_closing = self.text[node.opening + 1 : node.closing]
        if "\n" in before_closing or "\r" in before_closing:
            prefix = "  "
        else:
            prefix = f"\n{indentation}  "
        return self._replace(node.closing, node.closing, f"{prefix}{rendered}\n{indentation}")

    def _replace(self, start: int, end: int, replacement: str) -> JsoncDocument:
        return self.parse(f"{self.text[:start]}{replacement}{self.text[end:]}")


class _Parser:
    def __init__(self, text: str) -> None:
        self.text = text
        self.index = 0

    def parse_value(self) -> ValueNode:
        self.skip_trivia()
        start = self.index
        if start >= len(self.text):
            self.error("Expecting value")
        character = self.text[start]
        if character == "{":
            return self.parse_object()
        if character == "[":
            return self.parse_array()
        if character == '"':
            value, end = self.parse_string()
            return ValueNode(start, end, value)
        end = start
        while end < len(self.text) and self.text[end] not in " \t\r\n,]}":
            end += 1
        if end == start:
            self.error("Expecting value")
        raw = self.text[start:end]
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as error:
            self.error(error.msg, start + error.pos)
        self.index = end
        return ValueNode(start, end, value)

    def parse_object(self) -> ObjectNode:
        start = self.index
        self.index += 1
        members: list[ObjectMember] = []
        self.skip_trivia()
        while self.index < len(self.text) and self.text[self.index] != "}":
            member_start = self.index
            if self.text[self.index] != '"':
                self.error("Expecting property name enclosed in double quotes")
            key, _ = self.parse_string()
            self.skip_trivia()
            if self.index >= len(self.text) or self.text[self.index] != ":":
                self.error("Expecting ':' delimiter")
            self.index += 1
            value = self.parse_value()
            self.skip_trivia()
            comma = (
                self.index if self.index < len(self.text) and self.text[self.index] == "," else None
            )
            if comma is not None:
                self.index += 1
                self.skip_trivia()
            members.append(ObjectMember(key, member_start, value.end, value, comma))
            if comma is None:
                break
        if self.index >= len(self.text) or self.text[self.index] != "}":
            self.error("Expecting ',' delimiter")
        closing = self.index
        self.index += 1
        return ObjectNode(
            start,
            self.index,
            {member.key: member.value.value for member in members},
            start,
            closing,
            tuple(members),
        )

    def parse_array(self) -> ArrayNode:
        start = self.index
        self.index += 1
        elements: list[ArrayElement] = []
        self.skip_trivia()
        while self.index < len(self.text) and self.text[self.index] != "]":
            value = self.parse_value()
            self.skip_trivia()
            comma = (
                self.index if self.index < len(self.text) and self.text[self.index] == "," else None
            )
            if comma is not None:
                self.index += 1
                self.skip_trivia()
            elements.append(ArrayElement(value.start, value.end, value, comma))
            if comma is None:
                break
        if self.index >= len(self.text) or self.text[self.index] != "]":
            self.error("Expecting ',' delimiter")
        closing = self.index
        self.index += 1
        return ArrayNode(
            start,
            self.index,
            [element.value.value for element in elements],
            start,
            closing,
            tuple(elements),
        )

    def parse_string(self) -> tuple[str, int]:
        start = self.index
        self.index += 1
        escaped = False
        while self.index < len(self.text):
            character = self.text[self.index]
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                self.index += 1
                raw = self.text[start : self.index]
                try:
                    return json.loads(raw), self.index
                except json.JSONDecodeError as error:
                    self.error(error.msg, start + error.pos)
            elif character in "\r\n":
                self.error("Invalid control character", self.index)
            self.index += 1
        self.error("Unterminated string starting at", start)

    def skip_trivia(self) -> None:
        while self.index < len(self.text):
            if self.text[self.index] in " \t\r\n":
                self.index += 1
                continue
            if self.text.startswith("//", self.index):
                self.index += 2
                while self.index < len(self.text) and self.text[self.index] not in "\r\n":
                    self.index += 1
                continue
            if self.text.startswith("/*", self.index):
                start = self.index
                self.index = self.text.find("*/", self.index + 2)
                if self.index < 0:
                    self.error("Unterminated block comment", start)
                self.index += 2
                continue
            return

    def error(self, message: str, position: int | None = None) -> NoReturn:
        raise json.JSONDecodeError(message, self.text, self.index if position is None else position)


def _indent_at(text: str, position: int) -> str:
    line_start = text.rfind("\n", 0, position) + 1
    indentation = text[line_start:position]
    return indentation if indentation.isspace() else ""


def _container_indentation(text: str, node: ObjectNode | ArrayNode) -> str:
    indentation = _indent_at(text, node.closing)
    if indentation:
        return indentation
    line_start = text.rfind("\n", 0, node.start) + 1
    prefix = text[line_start : node.start]
    return prefix[: len(prefix) - len(prefix.lstrip(" \t"))]


def _render_value(value: Any, indentation: str) -> str:
    rendered = json.dumps(value, indent=2)
    lines = rendered.splitlines()
    return "\n".join((lines[0], *(f"{indentation}{line}" for line in lines[1:])))


def _object_at(node: ValueNode, start: int) -> ObjectNode | None:
    if isinstance(node, ObjectNode):
        if node.start == start:
            return node
        for member in node.members:
            found = _object_at(member.value, start)
            if found is not None:
                return found
    elif isinstance(node, ArrayNode):
        for element in node.elements:
            found = _object_at(element.value, start)
            if found is not None:
                return found
    return None


def _array_at(node: ValueNode, start: int) -> ArrayNode | None:
    if isinstance(node, ObjectNode):
        for member in node.members:
            found = _array_at(member.value, start)
            if found is not None:
                return found
    elif isinstance(node, ArrayNode):
        if node.start == start:
            return node
        for element in node.elements:
            found = _array_at(element.value, start)
            if found is not None:
                return found
    return None
