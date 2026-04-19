"""Tests for file-path input (str path and pathlib.Path) across all methods."""

from __future__ import annotations

from pathlib import Path

import pytest

import fast_xml_flattener as fxf
from fast_xml_flattener import to_object


@pytest.fixture
def xml_file(tmp_path: Path, nested_xml: str) -> Path:
    p = tmp_path / "data.xml"
    p.write_text(nested_xml, encoding="utf-8")
    return p


@pytest.fixture
def multi_file(tmp_path: Path, multi_record_xml: str) -> Path:
    p = tmp_path / "multi.xml"
    p.write_text(multi_record_xml, encoding="utf-8")
    return p


# ---------------------------------------------------------------------------
# Path object input
# ---------------------------------------------------------------------------


def test_to_json_path(xml_file: Path, nested_xml: str) -> None:
    assert fxf.to_json(xml_file) == fxf.to_json(nested_xml)


def test_to_flatten_json_path(xml_file: Path, nested_xml: str) -> None:
    assert fxf.to_flatten_json(xml_file) == fxf.to_flatten_json(nested_xml)


def test_to_dict_path(xml_file: Path, nested_xml: str) -> None:
    assert fxf.to_dict(xml_file) == fxf.to_dict(nested_xml)


def test_to_flatten_dict_path(xml_file: Path, nested_xml: str) -> None:
    assert fxf.to_flatten_dict(xml_file) == fxf.to_flatten_dict(nested_xml)


def test_to_csv_path(multi_file: Path, multi_record_xml: str) -> None:
    assert fxf.to_csv(multi_file) == fxf.to_csv(multi_record_xml)


def test_to_parquet_path(multi_file: Path, tmp_path: Path, multi_record_xml: str) -> None:
    out_path = Path(str(tmp_path / "out_path.parquet"))
    out_str = Path(str(tmp_path / "out_str.parquet"))
    fxf.to_parquet(multi_file, out_path)
    fxf.to_parquet(multi_record_xml, out_str)
    assert out_path.stat().st_size == out_str.stat().st_size


def test_to_object_path(xml_file: Path) -> None:
    obj = to_object(xml_file)
    assert obj.root.user.name == "Alice"
    assert obj.root.user.address.city == "Warsaw"


# ---------------------------------------------------------------------------
# str path input (string that doesn't start with '<')
# ---------------------------------------------------------------------------


def test_to_json_str_path(xml_file: Path, nested_xml: str) -> None:
    assert fxf.to_json(str(xml_file)) == fxf.to_json(nested_xml)


def test_to_dict_str_path(xml_file: Path, nested_xml: str) -> None:
    assert fxf.to_dict(str(xml_file)) == fxf.to_dict(nested_xml)


def test_to_object_str_path(xml_file: Path) -> None:
    obj = to_object(str(xml_file))
    assert obj.root.user.name == "Alice"


# ---------------------------------------------------------------------------
# str XML content still works (starts with '<')
# ---------------------------------------------------------------------------


def test_str_xml_content_unchanged(nested_xml: str) -> None:
    result = fxf.to_dict(nested_xml)
    assert result["root"]["user"]["name"] == "Alice"


def test_str_xml_with_leading_whitespace(nested_xml: str) -> None:
    result = fxf.to_dict("  \n" + nested_xml)
    assert result["root"]["user"]["name"] == "Alice"


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


def test_missing_file_raises(tmp_path: Path) -> None:
    with pytest.raises(OSError):
        fxf.to_dict(tmp_path / "nonexistent.xml")


def test_missing_file_str_raises(tmp_path: Path) -> None:
    with pytest.raises(OSError):
        fxf.to_dict(str(tmp_path / "nonexistent.xml"))


def test_invalid_xml_in_file_raises(tmp_path: Path) -> None:
    bad = tmp_path / "bad.xml"
    bad.write_text("<unclosed>", encoding="utf-8")
    with pytest.raises(ValueError):
        fxf.to_dict(bad)


def test_wrong_type_raises() -> None:
    with pytest.raises(TypeError):
        fxf.to_dict(123)  # type: ignore[arg-type]
