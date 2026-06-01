#!/usr/bin/env python3
"""Unit checks for packaged app smoke validators that do not require macOS."""

from __future__ import annotations

import json
from tempfile import TemporaryDirectory
from pathlib import Path

from package_contract import PACKAGED_SMOKE_REQUIRED_CHECK_IDS
from smoke_launch_macos_app import (
  PACKAGED_SMOKE_RECEIPT_KIND,
  PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION,
  validate_packaged_web_search_snapshot,
  write_packaged_smoke_receipt,
)


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


def web_search_items(attributes: dict[str, str]) -> list[dict]:
  return [
    {
      "kind": "assistantMessage",
      "title": "Pith",
      "content": "Packaged smoke local response.",
      "attributes": {
        "handoffKind": "webSearchSources",
        **attributes,
      },
    }
  ]


def valid_web_search_attributes() -> dict[str, str]:
  return {
    "webSearchSourceMode": "searchResultAttribution",
    "pageFetchPerformed": "false",
    "sourceSnapshotAvailable": "true",
    "sourceSnapshotKind": "searchResults",
    "sourceSnapshotResultCount": "1",
    "sourceSnapshotHash": "0123456789abcdef",
    "sourceSnapshot": "\n".join(
      [
        "1. Pith packaged web search fixture",
        "URL: https://example.com/pith-packaged-smoke",
        "Snippet: Deterministic packaged web search result.",
        "Provider: fixture",
      ]
    ),
    "sourceUrls": "https://example.com/pith-packaged-smoke",
    "sourceTitles": "Pith packaged web search fixture",
  }


def main() -> int:
  validate_packaged_web_search_snapshot(web_search_items(valid_web_search_attributes()))

  missing_snapshot = valid_web_search_attributes()
  missing_snapshot["sourceSnapshotAvailable"] = "false"
  assert_raises(
    lambda: validate_packaged_web_search_snapshot(web_search_items(missing_snapshot)),
    "packaged smoke should require source snapshots",
  )

  fetched_page = valid_web_search_attributes()
  fetched_page["pageFetchPerformed"] = "true"
  assert_raises(
    lambda: validate_packaged_web_search_snapshot(web_search_items(fetched_page)),
    "packaged smoke should not overstate page fetch depth",
  )

  wrong_hash = valid_web_search_attributes()
  wrong_hash["sourceSnapshotHash"] = "short"
  assert_raises(
    lambda: validate_packaged_web_search_snapshot(web_search_items(wrong_hash)),
    "packaged smoke should require stable snapshot hash shape",
  )

  with TemporaryDirectory(prefix="pith-smoke-receipt-") as root:
    receipt_path = Path(root) / "receipt.json"
    write_packaged_smoke_receipt(receipt_path)
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    if receipt["schemaVersion"] != PACKAGED_SMOKE_RECEIPT_SCHEMA_VERSION:
      raise AssertionError("packaged smoke receipt should record its schema")
    if receipt["kind"] != PACKAGED_SMOKE_RECEIPT_KIND:
      raise AssertionError("packaged smoke receipt should record its kind")
    if [item["id"] for item in receipt["checks"]] != [
      check_id for check_id in PACKAGED_SMOKE_REQUIRED_CHECK_IDS
    ]:
      raise AssertionError("packaged smoke receipt should record stable check ids")

  print("smoke launch helper tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
