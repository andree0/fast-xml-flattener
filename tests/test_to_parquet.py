"""Tests for `fast_xml_flattener.to_parquet`."""

from __future__ import annotations

from pathlib import Path

import fast_xml_flattener as fxf
import pyarrow.parquet as pq
import pytest


def test_file_is_created(simple_xml: str, tmp_path: Path) -> None:
    out = tmp_path / "single.parquet"
    fxf.to_parquet(simple_xml, str(out))
    assert out.exists() and out.stat().st_size > 0


def test_schema_and_values_single_record(simple_xml: str, tmp_path: Path) -> None:
    out = tmp_path / "single.parquet"
    fxf.to_parquet(simple_xml, str(out))
    table = pq.read_table(out)
    assert table.column_names == ["root.a", "root.b"]
    assert table.num_rows == 1
    assert table.to_pydict() == {"root.a": ["1"], "root.b": ["2"]}


def test_multi_record_rows(multi_record_xml: str, tmp_path: Path) -> None:
    out = tmp_path / "multi.parquet"
    fxf.to_parquet(multi_record_xml, str(out))
    table = pq.read_table(out)
    assert table.num_rows == 3
    assert table.column_names == ["id", "name"]
    assert table.to_pydict() == {
        "id": ["1", "2", "3"],
        "name": ["Alice", "Bob", "Charlie"],
    }


def test_missing_field_is_null(tmp_path: Path) -> None:
    xml = "<xs><x><a>1</a><b>2</b></x><x><a>3</a></x></xs>"
    out = tmp_path / "sparse.parquet"
    fxf.to_parquet(xml, str(out))
    table = pq.read_table(out)
    assert table.to_pydict() == {"a": ["1", "3"], "b": ["2", None]}


def test_include_attrs_true(attrs_xml: str, tmp_path: Path) -> None:
    out = tmp_path / "attrs.parquet"
    fxf.to_parquet(attrs_xml, str(out), include_attrs=True)
    table = pq.read_table(out)
    assert "item.@id" in table.column_names


def test_include_attrs_false(attrs_xml: str, tmp_path: Path) -> None:
    out = tmp_path / "noattrs.parquet"
    fxf.to_parquet(attrs_xml, str(out), include_attrs=False)
    table = pq.read_table(out)
    assert all("@" not in c for c in table.column_names)


def test_io_error_on_invalid_path(simple_xml: str, tmp_path: Path) -> None:
    # Writing to a non-existent directory path must raise OSError / IOError.
    with pytest.raises(OSError):
        fxf.to_parquet(simple_xml, str(tmp_path / "nope" / "x.parquet"))
