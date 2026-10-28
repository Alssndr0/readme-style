"""Line-level edits applied bottom-up, keeping one blank line around inserted blocks."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class _Op:
    start: int
    end: int
    lines: list[str]
    pad: bool


def _blank(line: str) -> bool:
    return not line.strip()


class Editor:
    def __init__(self, lines: list[str]) -> None:
        self._lines = list(lines)
        self._ops: list[_Op] = []

    def replace(self, start: int, end: int, lines: list[str], *, pad: bool = False) -> None:
        """Replace lines [start, end). With pad, keep blank lines around the new block."""
        self._ops.append(_Op(start, end, list(lines), pad))

    def insert(self, at: int, lines: list[str]) -> None:
        """Insert a block before line `at`, separated from its neighbours by blank lines."""
        for op in self._ops:
            if op.start == op.end == at:
                op.lines += ["", *lines]
                return
        self._ops.append(_Op(at, at, list(lines), True))

    def delete(self, start: int, end: int) -> None:
        """Delete lines [start, end) and collapse the blank lines left at the seam."""
        self._ops.append(_Op(start, end, [], True))

    @property
    def changed(self) -> bool:
        return bool(self._ops)

    def apply(self) -> list[str]:
        out = self._lines
        floor: int | None = None
        for op in sorted(self._ops, key=lambda o: (o.start, o.end), reverse=True):
            if floor is not None and op.end > floor:
                raise ValueError(f"overlapping edits at lines {op.start}-{op.end}")
            new = list(op.lines)
            if op.pad and new:
                if op.start > 0 and not _blank(out[op.start - 1]):
                    new.insert(0, "")
                if op.end < len(out) and not _blank(out[op.end]):
                    new.append("")
            out[op.start : op.end] = new
            if op.pad and not op.lines:
                s = op.start
                if s < len(out) and _blank(out[s]) and (s == 0 or _blank(out[s - 1])):
                    del out[s]
                elif s == len(out) and s > 0 and _blank(out[s - 1]):
                    del out[s - 1]
            floor = op.start
        return out
