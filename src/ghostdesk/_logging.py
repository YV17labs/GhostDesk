# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Logging configuration."""

import logging

import uvicorn.config

logger = logging.getLogger("ghostdesk")

_LOG_FMT = "%(asctime)s  %(levelname)-5s  %(name)s  %(message)s"
_LOG_DATEFMT = "%H:%M:%S"


def configure_logging() -> None:
    """Unify log format across the app, MCP SDK, and uvicorn."""
    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter(_LOG_FMT, datefmt=_LOG_DATEFMT))

    root = logging.getLogger()
    root.handlers = [handler]
    root.setLevel(logging.INFO)

    fmt_cfg = {"fmt": _LOG_FMT, "datefmt": _LOG_DATEFMT}
    if "formatters" not in uvicorn.config.LOGGING_CONFIG:
        logger.warning(
            "uvicorn.config.LOGGING_CONFIG missing 'formatters' key; "
            "log format may not be unified across all output"
        )
    else:
        for formatter in uvicorn.config.LOGGING_CONFIG["formatters"].values():
            formatter.update(fmt_cfg)
