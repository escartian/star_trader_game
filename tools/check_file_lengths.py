"""Check maintained source file sizes without scanning build outputs."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOTS = (ROOT / "crates", ROOT / "tools")
SOURCE_SUFFIXES = {".rs", ".py", ".sh"}
LIMIT = 500


def main():
    failures = []
    count = 0
    for source_root in SOURCE_ROOTS:
        for path in sorted(source_root.rglob("*")):
            if path.suffix not in SOURCE_SUFFIXES or not path.is_file():
                continue
            count += 1
            lines = len(path.read_text().splitlines())
            if lines > LIMIT:
                failures.append(f"{path.relative_to(ROOT)}: {lines} lines (limit {LIMIT})")
    if failures:
        print("Split these files by responsibility:\n" + "\n".join(failures))
        return 1
    print(f"Source length check passed: {count} files, at most {LIMIT} lines each.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
