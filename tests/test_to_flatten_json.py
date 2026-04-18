"""Tests for `fast_xml_flattener.to_flatten_json`."""

from __future__ import annotations

import json

import fast_xml_flattener as fxf


def test_dot_notation_default(nested_xml: str) -> None:
    data = json.loads(fxf.to_flatten_json(nested_xml))
    assert data == {
        "root.user.id": "1",
        "root.user.name": "Alice",
        "root.user.address.city": "Warsaw",
        "root.user.address.zip": "00-001",
    }


def test_array_indexing() -> None:
    data = json.loads(fxf.to_flatten_json("<r><i>a</i><i>b</i></r>"))
    assert data == {"r.i[0]": "a", "r.i[1]": "b"}


def test_custom_separator(nested_xml: str) -> None:
    data = json.loads(fxf.to_flatten_json(nested_xml, separator="_"))
    assert "root_user_name" in data
    assert data["root_user_name"] == "Alice"


def test_slash_separator() -> None:
    data = json.loads(fxf.to_flatten_json("<r><a><b>x</b></a></r>", separator="/"))
    assert data == {"r/a/b": "x"}


def test_attributes_flattened(attrs_xml: str) -> None:
    data = json.loads(fxf.to_flatten_json(attrs_xml))
    assert data == {
        "item.@id": "42",
        "item.@status": "open",
        "item.title": "Hello",
    }


def test_mixed_content_flatten() -> None:
    data = json.loads(fxf.to_flatten_json("<p>hello <b>world</b></p>"))
    assert data == {"p.#text": "hello ", "p.b": "world"}


def test_empty_element_produces_empty_string() -> None:
    data = json.loads(fxf.to_flatten_json("<r><x/></r>"))
    assert data == {"r.x": ""}
