from __future__ import annotations

from typing import List

from .cbor import CborError


class MarkdownError(ValueError):
    pass


MAX_HEADING_LEVEL = 6


def canonicalize_markdown_profile(text: str) -> str:
    normalized = text.replace("\r\n", "\n").replace("\r", "\n")
    out: List[str] = []
    blank_run = 0
    in_code = False

    for raw_line in normalized.split("\n"):
        line = raw_line.replace("\t", "    ").rstrip()

        if in_code:
            if _is_fence_line(line):
                out.append("```")
                in_code = False
                blank_run = 0
                continue
            out.append(line)
            blank_run = 0
            continue

        trimmed = line.strip()
        if trimmed == "":
            blank_run += 1
            if blank_run <= 2:
                out.append("")
            continue
        blank_run = 0

        if _is_fence_line(line):
            out.append(_canonical_fence(line))
            in_code = True
            continue

        out.append(_canonicalize_line(trimmed))

    if in_code:
        raise MarkdownError("unterminated code fence")

    return "\n".join(out)


def validate_markdown_profile(text: str) -> None:
    canonical = canonicalize_markdown_profile(text)
    if canonical != text:
        raise MarkdownError("markdown not canonical")


def _canonicalize_line(line: str) -> str:
    if line.startswith("    "):
        raise MarkdownError("indented code blocks not allowed")
    if _contains_html(line):
        raise MarkdownError("html not allowed")
    if "![" in line:
        raise MarkdownError("images not allowed")
    if _looks_like_table_separator(line):
        raise MarkdownError("tables not allowed")

    heading = _canonicalize_heading(line)
    if heading is not None:
        return heading
    blockquote = _canonicalize_blockquote(line)
    if blockquote is not None:
        return blockquote
    hr = _canonicalize_hr(line)
    if hr is not None:
        return hr
    list_line = _canonicalize_list(line)
    if list_line is not None:
        return list_line

    _validate_links(line)
    return line


def _is_fence_line(line: str) -> bool:
    return line.lstrip().startswith("```")


def _canonical_fence(line: str) -> str:
    trimmed = line.lstrip()
    rest = trimmed[3:]
    lang = rest.strip()
    if lang == "":
        return "```"
    if not all(c.isalnum() or c in "-_+." for c in lang):
        raise MarkdownError("invalid code fence language")
    return f"```{lang}"


def _canonicalize_heading(line: str) -> str | None:
    if not line.startswith("#"):
        return None
    level = 0
    for ch in line:
        if ch == "#":
            level += 1
        else:
            break
    if level == 0 or level > MAX_HEADING_LEVEL:
        raise MarkdownError("invalid heading")
    rest = line[level:].lstrip()
    if rest == "":
        raise MarkdownError("empty heading")
    return f"{'#' * level} {rest}"


def _canonicalize_blockquote(line: str) -> str | None:
    if not line.startswith(">"):
        return None
    rest = line[1:].lstrip()
    return f"> {rest}"


def _canonicalize_hr(line: str) -> str | None:
    if line.strip() == "---":
        return "---"
    return None


def _canonicalize_list(line: str) -> str | None:
    indent, rest = _split_indent(line)
    if indent > 3:
        raise MarkdownError("excessive indent")
    if rest.startswith("-"):
        if len(rest) == 1 or not rest[1].isspace():
            raise MarkdownError("invalid list marker")
        after = rest[1:].lstrip()
        if after == "":
            raise MarkdownError("invalid list marker")
        _validate_links(after)
        return f"{' ' * indent}- {after}"
    if rest.startswith("*") or rest.startswith("+"):
        raise MarkdownError("invalid list marker")
    if "." in rest:
        digits, after_dot = rest.split(".", 1)
        if digits.isdigit():
            if digits != "1":
                raise MarkdownError("ordered list must use 1.")
            if after_dot == "" or not after_dot[0].isspace():
                raise MarkdownError("invalid ordered list")
            after = after_dot.lstrip()
            if after == "":
                raise MarkdownError("invalid ordered list")
            _validate_links(after)
            return f"{' ' * indent}1. {after}"
    return None


def _split_indent(line: str) -> tuple[int, str]:
    count = 0
    for ch in line:
        if ch == " ":
            count += 1
        else:
            break
    return count, line[count:]


def _validate_links(line: str) -> None:
    idx = 0
    while True:
        start = line.find("](", idx)
        if start == -1:
            break
        url_start = start + 2
        end = line.find(")", url_start)
        if end == -1:
            raise MarkdownError("invalid link")
        url = line[url_start:end].strip()
        if ":" not in url:
            raise MarkdownError("invalid link")
        scheme = url.split(":", 1)[0].lower()
        if scheme not in {"https", "agentnet", "did"}:
            raise MarkdownError("invalid link scheme")
        idx = end + 1


def _contains_html(line: str) -> bool:
    for i, ch in enumerate(line):
        if ch == "<" and i + 1 < len(line):
            nxt = line[i + 1]
            if nxt.isalpha() or nxt in {"/", "!"}:
                return True
    return False


def _looks_like_table_separator(line: str) -> bool:
    if "|" not in line:
        return False
    trimmed = line.strip()
    if trimmed == "":
        return False
    for ch in trimmed:
        if ch not in "|:- ":
            return False
    return True
