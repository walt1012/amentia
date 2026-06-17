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
RELEASE_REPOSITORY = "walt1012/amentia"
RELEASE_REPOSITORY_URL = f"https://github.com/{RELEASE_REPOSITORY}"
RELEASE_ACTIONS_RUN_URL_PREFIX = f"{RELEASE_REPOSITORY_URL}/actions/runs/"


def release_actions_run_url(run_id: str | int) -> str:
  return f"{RELEASE_ACTIONS_RUN_URL_PREFIX}{run_id}"


def release_repository_url(path: str = "") -> str:
  suffix = path.strip("/")
  if not suffix:
    return RELEASE_REPOSITORY_URL
  return f"{RELEASE_REPOSITORY_URL}/{suffix}"


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
