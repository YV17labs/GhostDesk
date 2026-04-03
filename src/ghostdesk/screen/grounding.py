# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Detect UI elements via RapidOCR (PaddleOCR models via ONNX Runtime)."""

from __future__ import annotations

from dataclasses import dataclass, field

from rapidocr import RapidOCR

# Singleton OCR engine — loaded once, reused across calls.
_ocr: RapidOCR | None = None

# Minimum confidence to keep a detection.
_MIN_CONF = 0.3

# Minimum text height in pixels.
_MIN_HEIGHT = 10


def _get_ocr() -> RapidOCR:
    """Return the shared RapidOCR instance (lazy init)."""
    global _ocr  # noqa: PLW0603
    if _ocr is None:
        _ocr = RapidOCR()
        _ocr.text_score = _MIN_CONF
        _ocr.min_height = _MIN_HEIGHT
    return _ocr


@dataclass
class Element:
    """A detected UI element with position, label, and confidence."""

    id: int
    x: int
    y: int
    width: int
    height: int
    center_x: int = field(init=False)
    center_y: int = field(init=False)
    label: str = ""
    confidence: float = 0.0

    def __post_init__(self) -> None:
        self.center_x = self.x + self.width // 2
        self.center_y = self.y + self.height // 2

    def to_dict(self) -> dict:
        return {
            "label": self.label,
            "x": self.center_x,
            "y": self.center_y,
        }


def detect_elements(png_bytes: bytes) -> list[Element]:
    """Detect all visible UI elements via OCR.

    Returns elements sorted top-to-bottom, left-to-right.
    """
    ocr = _get_ocr()
    result = ocr(png_bytes)

    if not result:
        return []

    elements: list[Element] = []
    for box, text, conf in zip(result.boxes, result.txts, result.scores):
        x1 = int(min(p[0] for p in box))
        y1 = int(min(p[1] for p in box))
        x2 = int(max(p[0] for p in box))
        y2 = int(max(p[1] for p in box))

        elements.append(Element(
            id=0,
            x=x1,
            y=y1,
            width=x2 - x1,
            height=y2 - y1,
            label=text,
            confidence=float(conf),
        ))

    # Sort: top-to-bottom (by y, 15px bands), then left-to-right.
    elements.sort(key=lambda e: (e.y // 15, e.x))

    # Assign sequential IDs.
    for i, el in enumerate(elements, start=1):
        el.id = i

    return elements
