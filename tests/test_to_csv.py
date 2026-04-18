"""Tests for `fast_xml_flattener.to_csv`."""

from __future__ import annotations

import csv
import io

import fast_xml_flattener as fxf


def _parse_csv(text: str) -> tuple[list[str], list[list[str]]]:
    """Parse CSV text using the stdlib csv module for robust comparison."""
    reader = csv.reader(io.StringIO(text))
    header = next(reader)
    rows = list(reader)
    return header, rows


def test_single_record_header_and_row(simple_xml: str) -> None:
    header, rows = _parse_csv(fxf.to_csv(simple_xml))
    assert header == ["root.a", "root.b"]
    assert rows == [["1", "2"]]


def test_multi_record(multi_record_xml: str) -> None:
    header, rows = _parse_csv(fxf.to_csv(multi_record_xml))
    assert header == ["id", "name"]
    assert rows == [["1", "Alice"], ["2", "Bob"], ["3", "Charlie"]]


def test_missing_field_produces_empty_cell() -> None:
    xml = (
        "<xs>"
        "<x><a>1</a><b>2</b></x>"
        "<x><a>3</a></x>"  # no <b>
        "</xs>"
    )
    header, rows = _parse_csv(fxf.to_csv(xml))
    assert header == ["a", "b"]
    assert rows == [["1", "2"], ["3", ""]]


def test_include_attrs_true_by_default(attrs_xml: str) -> None:
    header, rows = _parse_csv(fxf.to_csv(attrs_xml))
    assert "item.@id" in header
    assert "item.@status" in header


def test_include_attrs_false(attrs_xml: str) -> None:
    header, rows = _parse_csv(fxf.to_csv(attrs_xml, include_attrs=False))
    assert all("@" not in col for col in header)
    assert header == ["item.title"]


def test_values_with_commas_are_quoted() -> None:
    output = fxf.to_csv("<r><a>hello, world</a></r>")
    assert '"hello, world"' in output


def test_values_with_quotes_are_escaped() -> None:
    output = fxf.to_csv('<r><a>she said "hi"</a></r>')
    _, rows = _parse_csv(output)
    assert rows[0][0] == 'she said "hi"'
