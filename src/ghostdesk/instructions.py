# Copyright (c) 2026 YV17 — MIT License
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop.

## How to interact

- **See the screen**: read_screen() — returns elements with role, name, and coordinates.
- **Click**: mouse_click(x, y) — use center_x, center_y from read_screen results.
- **Type text**: mouse_click(x, y) on the field, then type_text("hello").
- **Press keys**: press_key("Return"), press_key("ctrl+v").
- **Wait for page**: wait_for_element("expected text") — use after navigation.
- **Launch apps**: launch("firefox https://example.com").
- **Scroll**: mouse_scroll(x, y, direction="down", amount=3).
- **Screenshot**: screenshot() — only when read_screen() is not enough.

## 4 rules

1. **read_screen() first.** It returns `items` (app content with coordinates),
   `browser` (browser chrome), and `not_shown` (hidden element types).

2. **Use `not_shown` to find hidden content.** If read_screen() says
   `not_shown: ["table_row"]`, call `read_screen(role="table_row")`.

3. **Click with coordinates.** Each element has `center_x`, `center_y`.
   Use `mouse_click(center_x, center_y)` — fast and unambiguous.

4. **wait_for_element() after navigation.** After clicking a link or
   submitting a form, wait_for_element("expected text") tells you
   when the page is ready. Then read_screen() again.

## Example

```
read_screen()
# → items: [{role: "button", name: "Compose", center_x: 108, center_y: 185},
#            {role: "table_row", name: "Alice, Meeting tomorrow...", center_x: 740, center_y: 218}]
mouse_click(740, 218)
# → clicked the email
wait_for_element("Alice")
read_screen()
# → email content
```
"""
