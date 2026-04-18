"""Tests for to_object() and XmlObject dot-notation access."""

from __future__ import annotations

import pytest

import fast_xml_flattener as fxf
from fast_xml_flattener import XmlObject, to_object

# ---------------------------------------------------------------------------
# Basic access
# ---------------------------------------------------------------------------


def test_returns_xml_object(simple_xml):
    obj = to_object(simple_xml)
    assert isinstance(obj, XmlObject)


def test_leaf_element_via_dot(simple_xml):
    obj = to_object(simple_xml)
    assert obj.root.a == "1"
    assert obj.root.b == "2"


def test_nested_dot_access(nested_xml):
    obj = to_object(nested_xml)
    assert obj.root.user.name == "Alice"
    assert obj.root.user.address.city == "Warsaw"
    assert obj.root.user.address.zip == "00-001"


# ---------------------------------------------------------------------------
# Attributes
# ---------------------------------------------------------------------------


def test_attrs_returns_dict_without_prefix(attrs_xml):
    obj = to_object(attrs_xml)
    assert obj.item._attrs == {"id": "42", "status": "open"}


def test_attrs_empty_when_no_attributes(simple_xml):
    obj = to_object(simple_xml)
    assert obj.root._attrs == {}


def test_attrs_on_nested_element():
    xml = '<root><user id="1" role="admin"><name>Alice</name></user></root>'
    obj = to_object(xml)
    assert obj.root.user._attrs == {"id": "1", "role": "admin"}
    assert obj.root.user.name == "Alice"


# ---------------------------------------------------------------------------
# Text content
# ---------------------------------------------------------------------------


def test_text_property_pure_text(simple_xml):
    obj = to_object(simple_xml)
    assert obj.root.a._text == "1"


def test_text_property_mixed_content():
    xml = '<root><city country="PL">Warsaw</city></root>'
    obj = to_object(xml)
    assert obj.root.city._text == "Warsaw"
    assert obj.root.city._attrs == {"country": "PL"}


def test_text_property_none_when_no_text():
    xml = "<root><a><b>1</b></a></root>"
    obj = to_object(xml)
    assert obj.root.a._text is None


# ---------------------------------------------------------------------------
# raw property
# ---------------------------------------------------------------------------


def test_raw_pure_text_returns_str(simple_xml):
    obj = to_object(simple_xml)
    assert obj.root.a.raw == "1"


def test_raw_element_returns_dict(nested_xml):
    obj = to_object(nested_xml)
    raw = obj.root.user.raw
    assert isinstance(raw, dict)
    assert "name" in raw
    assert "address" in raw


def test_raw_root_returns_full_dict(simple_xml):
    obj = to_object(simple_xml)
    raw = obj.raw
    assert isinstance(raw, dict)
    assert "root" in raw


# ---------------------------------------------------------------------------
# Repeated elements → list
# ---------------------------------------------------------------------------


def test_repeated_elements_return_list(multi_record_xml):
    obj = to_object(multi_record_xml)
    users = obj.users.user
    assert isinstance(users, list)
    assert len(users) == 3
    assert all(isinstance(u, XmlObject) for u in users)


def test_repeated_elements_values(multi_record_xml):
    obj = to_object(multi_record_xml)
    names = [u.name for u in obj.users.user]
    assert names == ["Alice", "Bob", "Charlie"]


def test_repeated_elements_with_attrs():
    xml = '<root><item id="1">a</item><item id="2">b</item></root>'
    obj = to_object(xml)
    items = obj.root.item
    assert isinstance(items, list)
    assert items[0]._attrs == {"id": "1"}
    assert items[0]._text == "a"
    assert items[1]._attrs == {"id": "2"}
    assert items[1]._text == "b"


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


def test_missing_element_raises_attribute_error(simple_xml):
    obj = to_object(simple_xml)
    with pytest.raises(AttributeError, match="nonexistent"):
        _ = obj.root.nonexistent


def test_invalid_xml_raises():
    with pytest.raises(Exception):
        to_object("<unclosed>")


# ---------------------------------------------------------------------------
# __str__ and __repr__
# ---------------------------------------------------------------------------


def test_str_pure_text_leaf(simple_xml):
    obj = to_object(simple_xml)
    assert str(obj.root.a) == "1"


def test_str_node_returns_repr(simple_xml):
    obj = to_object(simple_xml)
    s = str(obj.root)
    assert s.startswith("XmlObject(")


def test_repr(simple_xml):
    obj = to_object(simple_xml)
    r = repr(obj)
    assert r.startswith("XmlObject(")


# ---------------------------------------------------------------------------
# Equality
# ---------------------------------------------------------------------------


def test_equal_objects(simple_xml):
    a = to_object(simple_xml)
    b = to_object(simple_xml)
    assert a == b


def test_unequal_objects(simple_xml, nested_xml):
    assert to_object(simple_xml) != to_object(nested_xml)


# ---------------------------------------------------------------------------
# Backward compatibility - Rust functions still importable from package
# ---------------------------------------------------------------------------


def test_to_dict_still_works(simple_xml):
    result = fxf.to_dict(simple_xml)
    assert isinstance(result, dict)
    assert result == {"root": {"a": "1", "b": "2"}}


def test_to_json_still_works(simple_xml):
    import json

    result = json.loads(fxf.to_json(simple_xml))
    assert result == {"root": {"a": "1", "b": "2"}}


def test_to_flatten_dict_still_works(simple_xml):
    result = fxf.to_flatten_dict(simple_xml)
    assert result["root.a"] == "1"


# ---------------------------------------------------------------------------
# Unicode / special content
# ---------------------------------------------------------------------------


def test_unicode_content():
    xml = "<root><city>Kraków</city></root>"
    obj = to_object(xml)
    assert obj.root.city == "Kraków"


def test_cdata_content():
    xml = "<root><note><![CDATA[Hello & World]]></note></root>"
    obj = to_object(xml)
    assert obj.root.note == "Hello & World"
