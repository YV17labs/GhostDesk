# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Detect UI elements in screenshots using the GPA-GUI-Detector model."""

from __future__ import annotations

import asyncio
from pathlib import Path
from typing import Any

from PIL import Image
from ultralytics import YOLO as _DetectorBackend

_model_cache: _DetectorBackend | None = None

# Model path: stored at the project root (excluded from git)
_MODEL_PATH = Path(__file__).resolve().parents[3] / "models" / "model.pt"

# Architectural stride of the detector backbone; input dimensions must be
# a multiple of this value.
_MODEL_STRIDE = 32
_NMS_IOU = 0.7


def _round_up_to_stride(value: int, stride: int = _MODEL_STRIDE) -> int:
    return ((value + stride - 1) // stride) * stride


def _get_model() -> _DetectorBackend:
    """Load the GPA-GUI-Detector model (lazy init).

    Loads the pre-downloaded model.pt from the models directory.
    """
    global _model_cache  # noqa: PLW0603
    if _model_cache is None:
        if not _MODEL_PATH.exists():
            raise FileNotFoundError(
                f"GPA-GUI-Detector model not found at {_MODEL_PATH}. "
                "Please ensure model.pt is packaged with the application."
            )
        _model_cache = _DetectorBackend(str(_MODEL_PATH))
    return _model_cache


def _run_detection(
    image: Image.Image,
    confidence: float,
) -> list[dict[str, Any]]:
    """Run GPA-GUI-Detector inference (blocking, meant for executor).

    Args:
        image: PIL Image to detect elements in
        confidence: Detector confidence threshold (0..1). Lower = more sensitive.

    Returns:
        List of detections in image-local coordinates.
    """
    model = _get_model()

    # Run inference at the image's native resolution (rounded up to the model
    # stride). Forcing a fixed imgsz like 640 distorts box coordinates.
    # Note: ultralytics expects imgsz as (height, width) when passed as a tuple.
    w, h = image.size
    results = model.predict(
        source=image,
        conf=confidence,
        imgsz=(_round_up_to_stride(h), _round_up_to_stride(w)),
        iou=_NMS_IOU,
        verbose=False,
    )

    result = results[0]
    detections = []

    boxes = result.boxes.xyxy.cpu().numpy()
    scores = result.boxes.conf.cpu().numpy()
    class_ids = result.boxes.cls.cpu().numpy().astype(int)
    names = model.names  # GPA-GUI-Detector has a single "icon" class.

    for box, score, cls_id in zip(boxes, scores, class_ids):
        x1, y1, x2, y2 = box
        label = names.get(int(cls_id), "ui")

        center_x = int((x1 + x2) / 2)
        center_y = int((y1 + y2) / 2)

        detections.append({
            "label": label,
            "box": [int(x1), int(y1), int(x2), int(y2)],
            "score": float(score),
            "center": [center_x, center_y],
        })

    return detections


async def detect(
    image: Image.Image,
    confidence: float,
) -> list[dict[str, Any]]:
    """Detect UI elements in an image using GPA-GUI-Detector.

    Runs detector inference asynchronously in a thread pool to avoid blocking
    the event loop. All coordinates returned are local to ``image`` — the
    caller is responsible for translating them to screen space if needed.

    Args:
        image: PIL Image to detect elements in
        confidence: Detector confidence threshold (0..1). Lower = more sensitive.

    Returns:
        List of detections, each with:
        - label: str (element type, e.g. "icon")
        - box: [x1, y1, x2, y2] (image-local coordinates)
        - score: float (confidence 0-1)
        - center: [cx, cy] (center point, image-local coordinates)
    """
    loop = asyncio.get_event_loop()
    return await loop.run_in_executor(
        None,
        _run_detection,
        image,
        confidence,
    )
