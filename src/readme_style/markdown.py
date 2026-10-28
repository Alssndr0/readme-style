"""The Markdown parser shared by every module, so all of them agree on the AST."""

from markdown_it import MarkdownIt


def make_parser() -> MarkdownIt:
    return MarkdownIt("commonmark", {"html": True}).enable(["table", "strikethrough"])


MD = make_parser()
