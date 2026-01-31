import json
import sys
from pathlib import Path

from .markdown import canonicalize_markdown_profile, validate_markdown_profile


def main():
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("spec/agentnet-markdown-tests-v0.1.json")
    data = json.loads(path.read_text())
    for case in data["cases"]:
        case_id = case["id"]
        input_text = case["input"]
        canonical = case["canonical"]
        valid = case["valid"]
        try:
            normalized = canonicalize_markdown_profile(input_text)
        except Exception:
            if canonical != "":
                raise AssertionError(f"{case_id} canonicalization failed")
        else:
            if normalized != canonical:
                raise AssertionError(f"{case_id} canonical mismatch")
        is_valid = True
        try:
            validate_markdown_profile(input_text)
        except Exception:
            is_valid = False
        if is_valid != valid:
            raise AssertionError(f"{case_id} validity mismatch")
    print("markdown profile tests complete")


if __name__ == "__main__":
    main()
