import { readFileSync } from "node:fs";
import { canonicalizeMarkdownProfile, validateMarkdownProfile } from "./markdown.js";

const path = process.argv[2] ?? "spec/agentnet-markdown-tests-v0.1.json";
const data = JSON.parse(readFileSync(path, "utf-8"));

for (const entry of data.cases as any[]) {
  const id = entry.id as string;
  const input = entry.input as string;
  const canonical = entry.canonical as string;
  const valid = entry.valid as boolean;

  try {
    const normalized = canonicalizeMarkdownProfile(input);
    if (normalized !== canonical) {
      throw new Error(`${id} canonical mismatch`);
    }
  } catch (err) {
    if (canonical !== "") {
      throw err;
    }
  }

  let isValid = true;
  try {
    validateMarkdownProfile(input);
  } catch {
    isValid = false;
  }
  if (isValid !== valid) {
    throw new Error(`${id} validity mismatch`);
  }
}

console.log("markdown profile tests complete");
