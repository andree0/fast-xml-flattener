"""Tests for `fast_xml_flattener.to_json` (1:1 structure preservation)."""

from __future__ import annotations

import json

import fast_xml_flattener as fxf


def test_pure_text_leaf_collapses_to_string() -> None:
    assert fxf.to_json("<a>hi</a>") == '{"a":"hi"}'


def test_nested_structure(nested_xml: str) -> None:
    data = json.loads(fxf.to_json(nested_xml))
    assert data == {
        "root": {
            "user": {
                "id": "1",
                "name": "Alice",
                "address": {"city": "Warsaw", "zip": "00-001"},
            }
        }
    }


def test_repeated_children_become_array() -> None:
    data = json.loads(fxf.to_json("<r><i>1</i><i>2</i><i>3</i></r>"))
    assert data == {"r": {"i": ["1", "2", "3"]}}


def test_attributes_use_at_prefix(attrs_xml: str) -> None:
    data = json.loads(fxf.to_json(attrs_xml))
    assert data == {"item": {"@id": "42", "@status": "open", "title": "Hello"}}


def test_mixed_content_uses_text_key() -> None:
    data = json.loads(fxf.to_json("<p>hello <b>world</b></p>"))
    # Text before the child element is captured under #text.
    assert data == {"p": {"#text": "hello ", "b": "world"}}


def test_empty_element() -> None:
    data = json.loads(fxf.to_json("<r><x/></r>"))
    assert data == {"r": {"x": {}}}


def test_self_closing_with_attrs() -> None:
    data = json.loads(fxf.to_json('<r><x a="1"/></r>'))
    assert data == {"r": {"x": {"@a": "1"}}}


def test_namespace_prefixes_stripped() -> None:
    xml = '<ns:r xmlns:ns="http://x"><ns:a>1</ns:a></ns:r>'
    data = json.loads(fxf.to_json(xml))
    assert data == {"r": {"a": "1"}}


def test_utf8_polish_diacritics() -> None:
    data = json.loads(fxf.to_json("<r><a>żółć</a></r>"))
    assert data == {"r": {"a": "żółć"}}


def test_cdata_preserves_raw_content() -> None:
    data = json.loads(fxf.to_json("<r><![CDATA[<raw>]]></r>"))
    assert data == {"r": "<raw>"}
