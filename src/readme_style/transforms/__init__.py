"""The transform pipeline. Each step parses the current text afresh and returns new lines."""

from __future__ import annotations

from collections.abc import Callable

from ..document import Document
from .base import Context
from .callouts import callouts
from .collapse import collapse
from .header import header
from .layout import center, strip_wrappers
from .logos import picture_logos
from .toc import toc

Transform = Callable[[Document, Context], "list[str] | None"]

PIPELINE: list[Transform] = [strip_wrappers, callouts, picture_logos, collapse, header, toc, center]

__all__ = ["PIPELINE", "Context", "Transform"]
