"""readme-style: keep your README's words, upgrade its presentation."""

__version__ = "0.1.0"

from .engine import GuardError, IdempotencyError, Result, style_file, style_text  # noqa: E402
from .themes import THEMES, Theme, get_theme  # noqa: E402

__all__ = [
    "THEMES",
    "GuardError",
    "IdempotencyError",
    "Result",
    "Theme",
    "__version__",
    "get_theme",
    "style_file",
    "style_text",
]
