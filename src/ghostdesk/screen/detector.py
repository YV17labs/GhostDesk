# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Detect UI elements in screenshots using GPA-GUI-Detector (YOLO-based)."""

from __future__ import annotations

import asyncio
from pathlib import Path
from typing import Any

from PIL import Image
from ultralytics import YOLO

_model_cache: YOLO | None = None

# Model path: packaged with the application
_MODEL_PATH = Path(__file__).parent.parent / "models" / "model.pt"


def _get_model() -> YOLO:
    """Load GPA-GUI-Detector YOLO model (lazy init).

    Loads the pre-downloaded model.pt from the models directory.
    """
    global _model_cache  # noqa: PLW0603
    if _model_cache is None:
        if not _MODEL_PATH.exists():
            raise FileNotFoundError(
                f"GPA-GUI-Detector model not found at {_MODEL_PATH}. "
                "Please ensure model.pt is packaged with the application."
            )
        _model_cache = YOLO(str(_MODEL_PATH))
    return _model_cache


def _run_detection(
    image: Image.Image,
    offset_x: int,
    offset_y: int,
) -> list[dict[str, Any]]:
    """Run GPA-GUI-Detector YOLO inference (blocking, meant for executor).

    Args:
        image: PIL Image to detect elements in
        offset_x: X offset to add to all box coordinates (for absolute screen coords)
        offset_y: Y offset to add to all box coordinates (for absolute screen coords)

    Returns:
        List of detections with label, box, score, and center coordinates
    """
    model = _get_model()

    # Run inference (YOLO returns Results object)
    results = model.predict(
        source=image,
        conf=0.05,        # Low confidence threshold to catch more elements
        imgsz=640,        # Standard input size for YOLO
        iou=0.7,          # NMS IoU threshold
        verbose=False,
    )

    result = results[0]
    detections = []

    # Extract boxes and scores from YOLO results
    boxes = result.boxes.xyxy.cpu().numpy()  # [x1, y1, x2, y2]
    scores = result.boxes.conf.cpu().numpy()  # confidence scores

    # YOLO model doesn't have class labels for UI elements
    # All detections are generic UI elements
    for box, score in zip(boxes, scores):
        x1, y1, x2, y2 = box

        # Determine element type based on aspect ratio (heuristic)
        width = x2 - x1
        height = y2 - y1
        aspect_ratio = width / height if height > 0 else 1

        if aspect_ratio > 2.5:
            label = "input"  # Wide elements are likely text inputs
        elif aspect_ratio < 0.4:
            label = "icon"   # Tall elements are likely icons
        elif height < 25:
            label = "text"   # Small elements are likely text
        else:
            label = "button" # Default to button

        center_x = int((x1 + x2) / 2)
        center_y = int((y1 + y2) / 2)

        detections.append({
            "label": label,
            "box": [
                int(x1) + offset_x,
                int(y1) + offset_y,
                int(x2) + offset_x,
                int(y2) + offset_y,
            ],
            "score": float(score),
            "center": [
                center_x + offset_x,
                center_y + offset_y,
            ],
        })

    return detections


async def detect(
    image: Image.Image,
    offset_x: int = 0,
    offset_y: int = 0,
) -> list[dict[str, Any]]:
    """Detect UI elements in an image using GPA-GUI-Detector.

    Runs YOLO inference asynchronously in a thread pool to avoid blocking the event loop.

    Args:
        image: PIL Image to detect elements in
        offset_x: X offset to add to all box coordinates (for absolute screen coords)
        offset_y: Y offset to add to all box coordinates (for absolute screen coords)

    Returns:
        List of detections, each with:
        - label: str (element type: button, input, icon, text)
        - box: [x1, y1, x2, y2] (absolute screen coordinates)
        - score: float (confidence 0-1)
        - center: [cx, cy] (center point in absolute screen coordinates)
    """
    loop = asyncio.get_event_loop()
    return await loop.run_in_executor(
        None,
        _run_detection,
        image,
        offset_x,
        offset_y,
    )
