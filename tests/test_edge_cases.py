"""Edge-case coverage: malformed input, unusual constructs, large payloads."""

from __future__ import annotations

import json

import fast_xml_flattener as fxf
import pytest


def test_malformed_xml_raises() -> None:
    with pytest.raises(ValueError):
        fxf.to_json("<a><b></a>")


def test_empty_string_raises() -> None:
    with pytest.raises(ValueError):
        fxf.to_json("")


def test_polish_diacritics_roundtrip() -> None:
    data = json.loads(fxf.to_json("<r><a>zażółć gęślą jaźń</a></r>"))
    assert data == {"r": {"a": "zażółć gęślą jaźń"}}


def test_cjk_roundtrip() -> None:
    data = json.loads(fxf.to_json("<r><a>你好世界</a></r>"))
    assert data == {"r": {"a": "你好世界"}}


def test_comments_are_ignored() -> None:
    data = json.loads(fxf.to_json("<r><!-- comment --><a>1</a></r>"))
    assert data == {"r": {"a": "1"}}


def test_processing_instructions_ignored() -> None:
    xml = '<?xml version="1.0"?><r><a>1</a></r>'
    data = json.loads(fxf.to_json(xml))
    assert data == {"r": {"a": "1"}}


def test_entity_references_decoded() -> None:
    data = json.loads(fxf.to_json("<r><a>1 &amp; 2 &lt; 3</a></r>"))
    assert data == {"r": {"a": "1 & 2 < 3"}}


def test_large_input_smoke() -> None:
    # ~1000 nested records — ensures no stack overflow / pathological behavior.
    inner = "".join(f"<item><id>{i}</id></item>" for i in range(1000))
    xml = f"<items>{inner}</items>"
    data = json.loads(fxf.to_json(xml))
    assert len(data["items"]["item"]) == 1000
    assert data["items"]["item"][999] == {"id": "999"}


def test_deeply_nested() -> None:
    # 50 levels of nesting
    open_tags = "".join(f"<l{i}>" for i in range(50))
    close_tags = "".join(f"</l{i}>" for i in reversed(range(50)))
    xml = f"{open_tags}x{close_tags}"
    result = fxf.to_json(xml)
    assert "x" in result
