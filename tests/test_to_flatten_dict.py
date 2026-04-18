"""Tests for `fast_xml_flattener.to_flatten_dict`."""

from __future__ import annotations

import json

import fast_xml_flattener as fxf


def test_returns_native_dict(nested_xml: str) -> None:
    result = fxf.to_flatten_dict(nested_xml)
    assert isinstance(result, dict)
    assert result == {
        "root.user.id": "1",
        "root.user.name": "Alice",
        "root.user.address.city": "Warsaw",
        "root.user.address.zip": "00-001",
    }


def test_parity_with_to_flatten_json(nested_xml: str) -> None:
    assert fxf.to_flatten_dict(nested_xml) == json.loads(
        fxf.to_flatten_json(nested_xml)
    )


def test_custom_separator_underscore() -> None:
    result = fxf.to_flatten_dict("<r><a><b>x</b></a></r>", separator="_")
    assert result == {"r_a_b": "x"}


def test_custom_separator_slash() -> None:
    result = fxf.to_flatten_dict("<r><a><b>x</b></a></r>", separator="/")
    assert result == {"r/a/b": "x"}


def test_array_indices_use_bracket_notation() -> None:
    result = fxf.to_flatten_dict("<r><i>a</i><i>b</i></r>")
    assert result == {"r.i[0]": "a", "r.i[1]": "b"}


def test_flatten_attributes(attrs_xml: str) -> None:
    result = fxf.to_flatten_dict(attrs_xml)
    assert result["item.@id"] == "42"
    assert result["item.@status"] == "open"
    assert result["item.title"] == "Hello"
