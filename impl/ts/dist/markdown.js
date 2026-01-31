const MAX_HEADING_LEVEL = 6;
export function canonicalizeMarkdownProfile(text) {
    let normalized = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
    const out = [];
    let blankRun = 0;
    let inCode = false;
    for (const rawLine of normalized.split("\n")) {
        let line = rawLine.replace(/\t/g, "    ").replace(/\s+$/g, "");
        if (inCode) {
            if (isFenceLine(line)) {
                out.push("```");
                inCode = false;
                blankRun = 0;
                continue;
            }
            out.push(line);
            blankRun = 0;
            continue;
        }
        const trimmed = line.trim();
        if (trimmed.length === 0) {
            blankRun += 1;
            if (blankRun <= 2)
                out.push("");
            continue;
        }
        blankRun = 0;
        if (isFenceLine(line)) {
            out.push(canonicalFence(line));
            inCode = true;
            continue;
        }
        out.push(canonicalizeLine(trimmed));
    }
    if (inCode)
        throw new Error("unterminated code fence");
    return out.join("\n");
}
export function validateMarkdownProfile(text) {
    const canonical = canonicalizeMarkdownProfile(text);
    if (canonical !== text)
        throw new Error("markdown not canonical");
}
function canonicalizeLine(line) {
    if (line.startsWith("    "))
        throw new Error("indented code blocks not allowed");
    if (containsHtml(line))
        throw new Error("html not allowed");
    if (line.includes("!["))
        throw new Error("images not allowed");
    if (looksLikeTableSeparator(line))
        throw new Error("tables not allowed");
    const heading = canonicalizeHeading(line);
    if (heading)
        return heading;
    const blockquote = canonicalizeBlockquote(line);
    if (blockquote)
        return blockquote;
    const hr = canonicalizeHr(line);
    if (hr)
        return hr;
    const listLine = canonicalizeList(line);
    if (listLine)
        return listLine;
    validateLinks(line);
    return line;
}
function isFenceLine(line) {
    return line.trimStart().startsWith("```");
}
function canonicalFence(line) {
    const trimmed = line.trimStart();
    const lang = trimmed.slice(3).trim();
    if (!lang)
        return "```";
    if (!/^[A-Za-z0-9_+\-.]+$/.test(lang))
        throw new Error("invalid code fence language");
    return "```" + lang;
}
function canonicalizeHeading(line) {
    if (!line.startsWith("#"))
        return null;
    let level = 0;
    while (level < line.length && line[level] === "#")
        level += 1;
    if (level === 0 || level > MAX_HEADING_LEVEL)
        throw new Error("invalid heading");
    const rest = line.slice(level).trimStart();
    if (!rest)
        throw new Error("empty heading");
    return "#".repeat(level) + " " + rest;
}
function canonicalizeBlockquote(line) {
    if (!line.startsWith(">"))
        return null;
    const rest = line.slice(1).trimStart();
    return "> " + rest;
}
function canonicalizeHr(line) {
    if (line.trim() === "---")
        return "---";
    return null;
}
function canonicalizeList(line) {
    const { indent, rest } = splitIndent(line);
    if (indent > 3)
        throw new Error("excessive indent");
    if (rest.startsWith("-")) {
        if (rest.length === 1 || !/\s/.test(rest[1]))
            throw new Error("invalid list marker");
        const after = rest.slice(1).trimStart();
        if (!after)
            throw new Error("invalid list marker");
        validateLinks(after);
        return " ".repeat(indent) + "- " + after;
    }
    if (rest.startsWith("*") || rest.startsWith("+")) {
        throw new Error("invalid list marker");
    }
    const dotIndex = rest.indexOf(".");
    if (dotIndex > 0) {
        const digits = rest.slice(0, dotIndex);
        const afterDot = rest.slice(dotIndex + 1);
        if (/^\d+$/.test(digits)) {
            if (digits !== "1")
                throw new Error("ordered list must use 1.");
            if (!afterDot || !/\s/.test(afterDot[0]))
                throw new Error("invalid ordered list");
            const after = afterDot.trimStart();
            if (!after)
                throw new Error("invalid ordered list");
            validateLinks(after);
            return " ".repeat(indent) + "1. " + after;
        }
    }
    return null;
}
function splitIndent(line) {
    let indent = 0;
    while (indent < line.length && line[indent] === " ")
        indent += 1;
    return { indent, rest: line.slice(indent) };
}
function validateLinks(line) {
    let idx = 0;
    while (true) {
        const start = line.indexOf("](", idx);
        if (start === -1)
            break;
        const urlStart = start + 2;
        const end = line.indexOf(")", urlStart);
        if (end === -1)
            throw new Error("invalid link");
        const url = line.slice(urlStart, end).trim();
        const colon = url.indexOf(":");
        if (colon === -1)
            throw new Error("invalid link");
        const scheme = url.slice(0, colon).toLowerCase();
        if (scheme !== "https" && scheme !== "agentnet" && scheme !== "did") {
            throw new Error("invalid link scheme");
        }
        idx = end + 1;
    }
}
function containsHtml(line) {
    for (let i = 0; i < line.length - 1; i += 1) {
        if (line[i] === "<") {
            const next = line[i + 1];
            if (/^[A-Za-z/!]$/.test(next))
                return true;
        }
    }
    return false;
}
function looksLikeTableSeparator(line) {
    if (!line.includes("|"))
        return false;
    const trimmed = line.trim();
    if (!trimmed)
        return false;
    for (const ch of trimmed) {
        if (ch !== "|" && ch !== "-" && ch !== ":" && ch !== " ")
            return false;
    }
    return true;
}
