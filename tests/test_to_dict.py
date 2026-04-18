"""Tests for `fast_xml_flattener.to_dict` (native Python dict output)."""

from __future__ import annotations

import json

import fast_xml_flattener as fxf


def test_returns_native_dict(simple_xml: str) -> None:
    result = fxf.to_dict(simple_xml)
    assert isinstance(result, dict)
    assert result == {"root": {"a": "1", "b": "2"}}


def test_parity_with_to_json(nested_xml: str) -> None:
    """to_dict must produce identical data to json.loads(to_json)."""
    assert fxf.to_dict(nested_xml) == json.loads(fxf.to_json(nested_xml))


def test_nested_values_are_dicts() -> None:
    result = fxf.to_dict("<r><a><b>x</b></a></r>")
    assert isinstance(result["r"], dict)
    assert isinstance(result["r"]["a"], dict)  # type: ignore[index]
    assert result["r"]["a"]["b"] == "x"  # type: ignore[index]


def test_repeated_children_become_list() -> None:
    result = fxf.to_dict("<r><i>1</i><i>2</i></r>")
    assert isinstance(result["r"]["i"], list)  # type: ignore[index]
    assert result["r"]["i"] == ["1", "2"]  # type: ignore[index]


def test_preserves_insertion_order() -> None:
    # Python 3.7+ dicts preserve insertion order; our Rust builder must too.
    result = fxf.to_dict("<r><z>1</z><a>2</a><m>3</m></r>")
    assert list(result["r"].keys()) == ["z", "a", "m"]  # type: ignore[union-attr]


def test_attribute_keys_have_at_prefix(attrs_xml: str) -> None:
    result = fxf.to_dict(attrs_xml)
    assert "@id" in result["item"]  # type: ignore[operator]
