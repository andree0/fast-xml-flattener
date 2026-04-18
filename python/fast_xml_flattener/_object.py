from __future__ import annotations

from typing import Any

from fast_xml_flattener import _fast_xml_flattener


def _wrap(val: object) -> XmlObject | list[Any]:
    if isinstance(val, dict):
        return XmlObject(val)
    if isinstance(val, list):
        return [_wrap(v) for v in val]
    return XmlObject(str(val))


class XmlObject:
    """Dot-notation access wrapper over a parsed XML node.

    Children are accessed as attributes (``obj.city``).
    Attributes of the XML element are exposed via ``_attrs`` (dict, no ``@`` prefix).
    Text content is exposed via ``_text``.
    The underlying raw dict is available via ``raw``.
    """

    __slots__ = ("_data",)

    def __init__(self, data: dict[str, Any] | str) -> None:
        object.__setattr__(
            self,
            "_data",
            {"#text": data} if isinstance(data, str) else data,
        )

    def __getattr__(self, name: str) -> Any:
        data: dict[str, Any] = object.__getattribute__(self, "_data")
        try:
            val = data[name]
        except KeyError:
            raise AttributeError(f"No XML element '{name}'") from None
        return _wrap(val)

    @property
    def _attrs(self) -> dict[str, str]:
        """XML attributes of this element, keyed without the ``@`` prefix."""
        data: dict[str, Any] = object.__getattribute__(self, "_data")
        return {k[1:]: v for k, v in data.items() if k.startswith("@")}

    @property
    def _text(self) -> str | None:
        """Text content (``#text``) of this element, or ``None``."""
        data: dict[str, Any] = object.__getattribute__(self, "_data")
        return data.get("#text")  # type: ignore[return-value]

    @property
    def raw(self) -> dict[str, Any] | str:
        """Underlying raw value from ``to_dict()``.

        Returns a plain ``str`` for pure-text leaf nodes,
        otherwise the full ``dict`` for this node.
        """
        data: dict[str, Any] = object.__getattribute__(self, "_data")
        t: str | None = data.get("#text")  # type: ignore[assignment]
        if t is not None and len(data) == 1:
            return t
        return data

    def __repr__(self) -> str:
        data: dict[str, Any] = object.__getattribute__(self, "_data")
        return f"XmlObject({data!r})"

    def __str__(self) -> str:
        t = self._text
        return t if t is not None else repr(self)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, XmlObject):
            return bool(
                object.__getattribute__(self, "_data") == object.__getattribute__(other, "_data")
            )
        if isinstance(other, str):
            return self._text == other
        return NotImplemented


def to_object(xml: str) -> XmlObject:
    """Parse *xml* and return the document root as an :class:`XmlObject`.

    Children, attributes, and text content are accessible via dot notation::

        obj = to_object('<root><user id="1"><name>Alice</name></user></root>')
        obj.root.user.name      # -> "Alice"
        obj.root.user._attrs    # -> {"id": "1"}
        obj.root.user.raw       # -> {"@id": "1", "name": "Alice"}
    """
    return XmlObject(_fast_xml_flattener.to_dict(xml))
