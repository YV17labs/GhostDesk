# Copyright (c) 2026 YV17 — MIT License
"""Command: table — extract a structured table."""

from __future__ import annotations

import argparse
import json
import sys

from ._helpers import _get_text_content
from ._tree import _find_on_desktop


def cmd_table(args: argparse.Namespace) -> None:
    """Extract a structured table."""
    table_obj = _find_on_desktop(text=args.text, role="table")

    if table_obj is None:
        json.dump({"error": "No table found" + (f" matching '{args.text}'" if args.text else "")}, sys.stdout)
        return

    try:
        table_iface = table_obj.get_table_iface()
        if table_iface is None:
            json.dump({"error": "Element found but has no table interface"}, sys.stdout)
            return
    except Exception:
        json.dump({"error": "Could not get table interface"}, sys.stdout)
        return

    try:
        n_rows = table_obj.get_n_rows()
        n_cols = table_obj.get_n_columns()
    except Exception:
        json.dump({"error": "Could not read table dimensions"}, sys.stdout)
        return

    # Cap to avoid huge outputs
    max_rows = min(n_rows, args.max_rows)
    max_cols = min(n_cols, 50)

    # Read headers
    headers = []
    for c in range(max_cols):
        try:
            header_cell = table_obj.get_column_header(c)
            if header_cell:
                text = _get_text_content(header_cell) or header_cell.get_name() or ""
                headers.append(text)
            else:
                headers.append("")
        except Exception:
            headers.append("")

    # Read rows
    rows = []
    for r in range(max_rows):
        row = []
        for c in range(max_cols):
            try:
                cell = table_obj.get_accessible_at(r, c)
                if cell:
                    text = _get_text_content(cell) or cell.get_name() or ""
                    row.append(text)
                else:
                    row.append("")
            except Exception:
                row.append("")
        rows.append(row)

    result = {
        "name": "",
        "rows": n_rows,
        "columns": n_cols,
        "headers": headers,
        "data": rows,
    }
    try:
        result["name"] = table_obj.get_name() or ""
    except Exception:
        pass

    if n_rows > max_rows:
        result["truncated"] = True
        result["showing_rows"] = max_rows

    json.dump(result, sys.stdout, ensure_ascii=False)
