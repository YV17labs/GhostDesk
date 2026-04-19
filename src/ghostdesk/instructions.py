# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop through screen capture, mouse, and
keyboard. Each tool's docstring covers its own parameters and caveats;
the notes below are the cross-tool orchestration patterns a docstring
cannot express.

## Before the first click

Know what's installed. Call `app_list()` (or read `ghostdesk://apps`)
the first time a task names an application — its `exec` field is the
only string `app_launch()` will accept. Re-call it after installing
software during the session.

Know what's on screen. Coordinates in any mouse call are valid only
against the latest `screen_shot()`. The moment a window closes, a menu
opens, or the page scrolls, every prior reading is stale.

## Reading the feedback every action returns

Input tools return `{screen_changed, reaction_time_ms}`.

`screen_changed: true` means pixels around the action point moved
within 2 s — the input landed. It does NOT prove the app did the right
thing. Before anything irreversible (delete, send, close, pay, commit,
reply), take a fresh `screen_shot()` and verify the pixels match the
intent.

`screen_changed: false` means the input had no visible effect. Do not
retry the same coordinates or the same keystroke — the target moved, a
modal opened, or focus shifted. Take a new screenshot and recompute.

## Two paths for text entry

`key_type()` is for short, focused strings. For anything longer than a
sentence, `clipboard_set(text)` followed by `key_press("ctrl+v")` is
faster and immune to autocomplete or autocorrect interference.

## Finishing the mission

Before you declare a task done, take one last `screen_shot()` and
verify the end state against the original request. This self-check
catches the silent failures the action loop missed — wrong tab focused,
dialog left open, form field blank, file saved under the wrong name.
"""
