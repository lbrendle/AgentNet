import { readFileSync } from "node:fs";
import { canonicalizeMarkdownProfile, validateMarkdownProfile } from "./markdown.js";
const path = process.argv[2] ?? "spec/agentnet-markdown-tests-v0.1.json";
const data = JSON.parse(readFileSync(path, "utf-8"));
for (const entry of data.cases) {
    const id = entry.id;
    const input = entry.input;
    const canonical = entry.canonical;
    const valid = entry.valid;
    try {
        const normalized = canonicalizeMarkdownProfile(input);
        if (normalized !== canonical) {
            throw new Error(`${id} canonical mismatch`);
        }
    }
    catch (err) {
        if (canonical !== "") {
            throw err;
        }
    }
    let isValid = true;
    try {
        validateMarkdownProfile(input);
    }
    catch {
        isValid = false;
    }
    if (isValid !== valid) {
        throw new Error(`${id} validity mismatch`);
    }
}
console.log("markdown profile tests complete");
