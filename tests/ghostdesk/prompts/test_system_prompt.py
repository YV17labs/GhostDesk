# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.prompts.system_prompt."""

from ghostdesk.prompts.system_prompt import SYSTEM_PROMPT, system_prompt
from ghostdesk.server import create_app


async def test_system_prompt_returns_inlined_constant():
    """The prompt function returns the SYSTEM_PROMPT constant verbatim."""
    assert await system_prompt() == SYSTEM_PROMPT


def test_system_prompt_has_core_rule():
    """The system prompt still encodes the SEE → ACT → SEE contract."""
    # Spot-check core invariants that agents rely on; these being present is
    # what makes the prompt worth shipping as the recommended baseline.
    assert "SEE → ACT → SEE" in SYSTEM_PROMPT
    assert "screen_shot()" in SYSTEM_PROMPT


async def test_create_app_registers_system_prompt():
    """create_app() registers the system_prompt prompt."""
    app = create_app(port=9999)
    prompts = app._prompt_manager._prompts
    assert "system_prompt" in prompts, f"expected system_prompt, got {sorted(prompts)}"
