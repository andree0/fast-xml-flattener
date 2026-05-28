from os import PathLike
from pathlib import Path

__version__: str

XmlInput = str | Path | PathLike[str]

def to_json(
    xml: XmlInput,
    *,
    strip_whitespace: bool = True,
    keep_namespace_declarations: bool = False,
) -> str: ...
def to_flatten_json(
    xml: XmlInput,
    separator: str = ".",
    *,
    strip_whitespace: bool = True,
    keep_namespace_declarations: bool = False,
    index_as_key: bool = False,
) -> str: ...
def to_dict(
    xml: XmlInput,
    *,
    strip_whitespace: bool = True,
    keep_namespace_declarations: bool = False,
) -> dict: ...
def to_flatten_dict(
    xml: XmlInput,
    separator: str = ".",
    *,
    strip_whitespace: bool = True,
    keep_namespace_declarations: bool = False,
    index_as_key: bool = False,
) -> dict: ...
def to_csv(
    xml: XmlInput,
    include_attrs: bool = True,
    *,
    strip_whitespace: bool = True,
    keep_namespace_declarations: bool = False,
    index_as_key: bool = False,
) -> str: ...
def to_parquet(
    xml: XmlInput,
    path: str | Path,
    include_attrs: bool = True,
    *,
    strip_whitespace: bool = True,
    keep_namespace_declarations: bool = False,
    index_as_key: bool = False,
) -> None: ...
def get_root_tag_name(xml: XmlInput) -> str: ...
