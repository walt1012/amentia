#!/usr/bin/env python3
"""Shared release identity rules for Amentia package and release helpers."""

from __future__ import annotations

import re


PRODUCT_VERSION_PATTERN = re.compile(
  r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
)
PUBLIC_RELEASE_TAG_PATTERN = re.compile(
  r"^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
)


def normalize_product_version(value: str) -> str:
  version = value.strip()
  if version.startswith("v"):
    version = version[1:]
  if not PRODUCT_VERSION_PATTERN.fullmatch(version):
    raise RuntimeError(
      "Product version must be three dot-separated non-negative integers, "
      f"got {value!r}"
    )
  return version


def validate_public_release_tag(tag: str) -> None:
  if not PUBLIC_RELEASE_TAG_PATTERN.fullmatch(tag):
    raise RuntimeError(f"Public release tag must use vX.Y.Z format: {tag!r}")


def product_version_from_tag(tag: str) -> str:
  validate_public_release_tag(tag)
  return tag[1:]
