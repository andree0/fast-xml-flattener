"""Integration tests for the new parser/flatten options:

- strip_whitespace
- keep_namespace_declarations
- index_as_key
- get_root_tag_name
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import fast_xml_flattener as fxf

# ---------------------------------------------------------------------------
# strip_whitespace
# ---------------------------------------------------------------------------


class TestStripWhitespace:
    def test_default_true_trims_leading_trailing(self) -> None:
        d = fxf.to_dict("<a>  hello  </a>")
        assert d == {"a": "hello"}

    def test_false_preserves_text_verbatim(self) -> None:
        d = fxf.to_dict("<a>  hello  </a>", strip_whitespace=False)
        assert d == {"a": "  hello  "}

    def test_false_keeps_whitespace_only_text_between_elements(self) -> None:
        d = fxf.to_dict("<r>\n  <a>x</a>\n</r>", strip_whitespace=False)
        # With strip_whitespace=False, whitespace-only text between elements
        # is preserved as a #text node.
        assert d == {"r": {"#text": "\n  \n", "a": "x"}}

    def test_true_drops_whitespace_only_text_between_elements(self) -> None:
        d = fxf.to_dict("<r>\n  <a>x</a>\n</r>")
        assert d == {"r": {"a": "x"}}

    def test_entity_references_unaffected_by_trimming(self) -> None:
        # Regression: trimming must happen on the combined text node,
        # not on individual fragments between entity refs.
        d = fxf.to_dict("<r><a>1 &amp; 2 &lt; 3</a></r>")
        assert d == {"r": {"a": "1 & 2 < 3"}}

    def test_applies_to_flatten_dict(self) -> None:
        d = fxf.to_flatten_dict("<r><a>  x  </a></r>")
        assert d == {"r.a": "x"}
        d2 = fxf.to_flatten_dict("<r><a>  x  </a></r>", strip_whitespace=False)
        assert d2 == {"r.a": "  x  "}

    def test_applies_to_json(self) -> None:
        out = json.loads(fxf.to_json("<a>  hi  </a>"))
        assert out == {"a": "hi"}
        out2 = json.loads(fxf.to_json("<a>  hi  </a>", strip_whitespace=False))
        assert out2 == {"a": "  hi  "}

    def test_applies_to_csv(self) -> None:
        out = fxf.to_csv("<r><a>  x  </a></r>")
        # Header line + value line — value should be trimmed.
        assert "  x  " not in out
        assert "x" in out


# ---------------------------------------------------------------------------
# keep_namespace_declarations
# ---------------------------------------------------------------------------


class TestKeepNamespaceDeclarations:
    XML = '<tns:root xmlns:tns="http://ex.com" xmlns="http://def.com"><tns:a>1</tns:a></tns:root>'

    def test_default_false_strips_xmlns_attrs(self) -> None:
        d = fxf.to_dict(self.XML)
        # No xmlns* keys present.
        assert all(not k.startswith("@xmlns") for k in d["root"])

    def test_true_keeps_prefixed_namespace(self) -> None:
        d = fxf.to_dict(self.XML, keep_namespace_declarations=True)
        assert d["root"]["@xmlns:tns"] == "http://ex.com"

    def test_true_keeps_default_namespace(self) -> None:
        d = fxf.to_dict(self.XML, keep_namespace_declarations=True)
        assert d["root"]["@xmlns"] == "http://def.com"

    def test_tag_name_remains_local_only(self) -> None:
        d = fxf.to_dict(self.XML, keep_namespace_declarations=True)
        # Tag is still "root" (no "tns:" prefix), even when keeping xmlns.
        assert "root" in d
        assert "tns:root" not in d
        assert "a" in d["root"]

    def test_applies_to_flatten_dict(self) -> None:
        d = fxf.to_flatten_dict(self.XML, keep_namespace_declarations=True)
        assert d["root.@xmlns:tns"] == "http://ex.com"
        assert d["root.@xmlns"] == "http://def.com"

    def test_applies_to_json(self) -> None:
        out = json.loads(fxf.to_json(self.XML, keep_namespace_declarations=True))
        assert out["root"]["@xmlns:tns"] == "http://ex.com"


# ---------------------------------------------------------------------------
# index_as_key
# ---------------------------------------------------------------------------


class TestIndexAsKey:
    XML_REPEATED = "<r><a>0</a><i>1</i><i>2</i><i>3</i></r>"

    def test_default_uses_bracket_notation(self) -> None:
        d = fxf.to_flatten_dict(self.XML_REPEATED)
        assert d["r.i[0]"] == "1"
        assert d["r.i[2]"] == "3"

    def test_true_uses_separator_as_index_join(self) -> None:
        d = fxf.to_flatten_dict(self.XML_REPEATED, index_as_key=True)
        assert d["r.i.0"] == "1"
        assert d["r.i.1"] == "2"
        assert d["r.i.2"] == "3"
        assert "r.i[0]" not in d

    def test_custom_separator_propagates_to_index(self) -> None:
        d = fxf.to_flatten_dict(self.XML_REPEATED, separator=">", index_as_key=True)
        assert d["r>i>0"] == "1"
        assert d["r>i>2"] == "3"

    def test_applies_to_flatten_json(self) -> None:
        out = json.loads(fxf.to_flatten_json(self.XML_REPEATED, index_as_key=True))
        assert out["r.i.0"] == "1"
        assert out["r.i.2"] == "3"

    def test_applies_to_csv(self) -> None:
        out = fxf.to_csv(self.XML_REPEATED, index_as_key=True)
        header = out.splitlines()[0]
        assert "r.i.0" in header
        assert "r.i[0]" not in header

    def test_underscore_separator_with_index_as_key(self) -> None:
        d = fxf.to_flatten_dict(self.XML_REPEATED, separator="_", index_as_key=True)
        assert d["r_i_0"] == "1"

    # --- index in the MIDDLE of the path (children under the indexed node) ---

    def test_index_in_middle_with_children(self) -> None:
        xml = (
            "<users>"
            "<user><name>Alice</name><city>Warsaw</city></user>"
            "<user><name>Bob</name><city>Berlin</city></user>"
            "</users>"
        )
        # With a single-child root containing repeated tags, the flat dict
        # still includes the root prefix and the repeated tag is mid-path.
        d = fxf.to_flatten_dict(xml, index_as_key=True)
        assert d["users.user.0.name"] == "Alice"
        assert d["users.user.0.city"] == "Warsaw"
        assert d["users.user.1.name"] == "Bob"
        assert d["users.user.1.city"] == "Berlin"

    def test_index_in_middle_default_bracket_notation(self) -> None:
        xml = "<users><user><name>Alice</name></user><user><name>Bob</name></user></users>"
        d = fxf.to_flatten_dict(xml)
        assert d["users.user[0].name"] == "Alice"
        assert d["users.user[1].name"] == "Bob"

    def test_index_in_middle_with_attrs_after(self) -> None:
        xml = '<r><item id="1"><name>a</name></item><item id="2"><name>b</name></item></r>'
        d = fxf.to_flatten_dict(xml, index_as_key=True)
        assert d["r.item.0.@id"] == "1"
        assert d["r.item.0.name"] == "a"
        assert d["r.item.1.@id"] == "2"
        assert d["r.item.1.name"] == "b"

    def test_nested_lists_indices_at_two_levels(self) -> None:
        # Each <group> has multiple <item> children — both levels get indexed.
        xml = (
            "<r>"
            "<group><item>g0i0</item><item>g0i1</item></group>"
            "<group><item>g1i0</item><item>g1i1</item></group>"
            "</r>"
        )
        d = fxf.to_flatten_dict(xml, index_as_key=True)
        assert d["r.group.0.item.0"] == "g0i0"
        assert d["r.group.0.item.1"] == "g0i1"
        assert d["r.group.1.item.0"] == "g1i0"
        assert d["r.group.1.item.1"] == "g1i1"

    def test_nested_lists_default_bracket_notation(self) -> None:
        xml = (
            "<r>"
            "<group><item>g0i0</item><item>g0i1</item></group>"
            "<group><item>g1i0</item><item>g1i1</item></group>"
            "</r>"
        )
        d = fxf.to_flatten_dict(xml)
        assert d["r.group[0].item[0]"] == "g0i0"
        assert d["r.group[0].item[1]"] == "g0i1"
        assert d["r.group[1].item[0]"] == "g1i0"
        assert d["r.group[1].item[1]"] == "g1i1"

    def test_per_parent_list_detection_documented(self) -> None:
        # The library detects lists per-parent: if a sibling tag appears only
        # once inside a given parent it stays scalar (Children::One), even when
        # the same tag is a list elsewhere in the document. The index_as_key
        # flag only adds indices where the parser actually has a list.
        xml = (
            "<r>"
            "<group><item>g0i0</item><item>g0i1</item></group>"
            "<group><item>g1i0</item></group>"
            "</r>"
        )
        d = fxf.to_flatten_dict(xml, index_as_key=True)
        assert d["r.group.0.item.0"] == "g0i0"
        assert d["r.group.0.item.1"] == "g0i1"
        # Second group has a SINGLE item — no index in the key.
        assert d["r.group.1.item"] == "g1i0"

    def test_deep_path_after_index(self) -> None:
        xml = (
            "<r>"
            "<u><addr><city>Warsaw</city><zip>00-001</zip></addr></u>"
            "<u><addr><city>Berlin</city><zip>10115</zip></addr></u>"
            "</r>"
        )
        d = fxf.to_flatten_dict(xml, index_as_key=True)
        assert d["r.u.0.addr.city"] == "Warsaw"
        assert d["r.u.0.addr.zip"] == "00-001"
        assert d["r.u.1.addr.city"] == "Berlin"

    def test_nested_lists_with_custom_separator(self) -> None:
        xml = "<r><g><i>g0i0</i><i>g0i1</i></g><g><i>g1i0</i><i>g1i1</i></g></r>"
        d = fxf.to_flatten_dict(xml, separator=">", index_as_key=True)
        assert d["r>g>0>i>0"] == "g0i0"
        assert d["r>g>0>i>1"] == "g0i1"
        assert d["r>g>1>i>0"] == "g1i0"
        assert d["r>g>1>i>1"] == "g1i1"

    def test_csv_record_extraction_with_nested_indices(self) -> None:
        # Multi-record root: each <user> becomes a row; nested repeated
        # <hobby> inside a user gets the index_as_key treatment.
        xml = (
            "<users>"
            "<user><name>Alice</name><hobby>chess</hobby><hobby>go</hobby></user>"
            "<user><name>Bob</name><hobby>swim</hobby></user>"
            "</users>"
        )
        out = fxf.to_csv(xml, index_as_key=True)
        header = out.splitlines()[0]
        # Each row's columns include the dotted index for the inner list.
        assert "hobby.0" in header
        assert "hobby.1" in header
        assert "hobby[0]" not in header

    def test_flatten_json_index_in_middle(self) -> None:
        xml = "<r><item><k>a</k></item><item><k>b</k></item></r>"
        out = json.loads(fxf.to_flatten_json(xml, index_as_key=True))
        assert out["r.item.0.k"] == "a"
        assert out["r.item.1.k"] == "b"


# ---------------------------------------------------------------------------
# get_root_tag_name
# ---------------------------------------------------------------------------


class TestGetRootTagName:
    def test_simple_root(self) -> None:
        assert fxf.get_root_tag_name("<root><a/></root>") == "root"

    def test_strips_namespace_prefix(self) -> None:
        xml = '<tns:root xmlns:tns="http://x"><tns:a>1</tns:a></tns:root>'
        assert fxf.get_root_tag_name(xml) == "root"

    def test_strips_prefix_when_keep_namespace_unrelated(self) -> None:
        # get_root_tag_name does NOT take keep_namespace_declarations — it
        # always returns the local name without prefix.
        xml = '<a:item xmlns:a="http://x"/>'
        assert fxf.get_root_tag_name(xml) == "item"

    def test_with_xml_prolog(self) -> None:
        xml = '<?xml version="1.0" encoding="UTF-8"?><doc/>'
        assert fxf.get_root_tag_name(xml) == "doc"

    def test_with_comment_before_root(self) -> None:
        xml = "<!-- header --><root/>"
        assert fxf.get_root_tag_name(xml) == "root"

    def test_from_file(self, tmp_path: Path) -> None:
        p = tmp_path / "sample.xml"
        p.write_text('<?xml version="1.0"?><tns:catalog xmlns:tns="http://x"/>')
        assert fxf.get_root_tag_name(p) == "catalog"

    def test_from_str_path(self, tmp_path: Path) -> None:
        p = tmp_path / "sample.xml"
        p.write_text("<library/>")
        assert fxf.get_root_tag_name(str(p)) == "library"

    def test_empty_xml_raises(self) -> None:
        with pytest.raises(Exception):
            fxf.get_root_tag_name("")

    def test_missing_file_raises(self) -> None:
        with pytest.raises(Exception):
            fxf.get_root_tag_name("/definitely/does/not/exist_xyz.xml")


# ---------------------------------------------------------------------------
# Composed: multiple options together
# ---------------------------------------------------------------------------


class TestCombined:
    def test_all_options_together_for_flatten_dict(self) -> None:
        xml = (
            '<tns:root xmlns:tns="http://x">'
            "<tns:a>  hi  </tns:a>"
            "<tns:i>1</tns:i>"
            "<tns:i>2</tns:i>"
            "</tns:root>"
        )
        d = fxf.to_flatten_dict(
            xml,
            separator=".",
            strip_whitespace=True,
            keep_namespace_declarations=True,
            index_as_key=True,
        )
        assert d["root.a"] == "hi"
        assert d["root.@xmlns:tns"] == "http://x"
        assert d["root.i.0"] == "1"
        assert d["root.i.1"] == "2"

    def test_to_object_with_strip_whitespace(self) -> None:
        obj = fxf.to_object("<r><a>  hi  </a></r>")
        assert obj.r.a == "hi"
        obj2 = fxf.to_object("<r><a>  hi  </a></r>", strip_whitespace=False)
        assert obj2.r.a == "  hi  "

    def test_to_object_with_keep_namespace_declarations(self) -> None:
        xml = '<root xmlns:tns="http://x"><a>1</a></root>'
        obj = fxf.to_object(xml, keep_namespace_declarations=True)
        assert obj.root._attrs["xmlns:tns"] == "http://x"
