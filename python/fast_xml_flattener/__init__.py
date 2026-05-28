"""fast-xml-flattener - high-performance XML parsing with Python bindings."""

from __future__ import annotations

from fast_xml_flattener import _fast_xml_flattener
from fast_xml_flattener._object import XmlObject, to_object

# Re-export all Rust functions unchanged
to_json = _fast_xml_flattener.to_json
to_flatten_json = _fast_xml_flattener.to_flatten_json
to_dict = _fast_xml_flattener.to_dict
to_flatten_dict = _fast_xml_flattener.to_flatten_dict
to_csv = _fast_xml_flattener.to_csv
to_parquet = _fast_xml_flattener.to_parquet
get_root_tag_name = _fast_xml_flattener.get_root_tag_name

__all__ = [
    "XmlObject",
    "get_root_tag_name",
    "to_csv",
    "to_dict",
    "to_flatten_dict",
    "to_flatten_json",
    "to_json",
    "to_object",
    "to_parquet",
]
