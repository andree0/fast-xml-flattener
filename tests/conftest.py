"""Shared fixtures for the pytest suite."""

from __future__ import annotations

import pytest


@pytest.fixture
def simple_xml() -> str:
    """Minimal single-record XML with two leaf fields."""
    return "<root><a>1</a><b>2</b></root>"


@pytest.fixture
def nested_xml() -> str:
    """Nested XML: one user with address block."""
    return (
        "<root>"
        "<user>"
        "<id>1</id>"
        "<name>Alice</name>"
        "<address><city>Warsaw</city><zip>00-001</zip></address>"
        "</user>"
        "</root>"
    )


@pytest.fixture
def multi_record_xml() -> str:
    """Multiple records under a container — CSV/Parquet target shape."""
    return (
        "<users>"
        "<user><id>1</id><name>Alice</name></user>"
        "<user><id>2</id><name>Bob</name></user>"
        "<user><id>3</id><name>Charlie</name></user>"
        "</users>"
    )


@pytest.fixture
def attrs_xml() -> str:
    """XML with attributes and a text child."""
    return '<item id="42" status="open"><title>Hello</title></item>'
