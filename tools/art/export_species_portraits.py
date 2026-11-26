"""Resize the original species portraits into embedded game textures.

Run from the repository root with ``python3 tools/art/export_species_portraits.py``.
Requires Pillow. The 1254 px generated originals remain in assets/source/species.
"""

from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "assets" / "source" / "species"
DESTINATION = ROOT / "assets" / "portraits"


def main() -> None:
    DESTINATION.mkdir(parents=True, exist_ok=True)
    for name in ("human", "miri", "auralith", "kethari"):
        with Image.open(SOURCE / f"{name}.png") as original:
            portrait = original.convert("RGB")
            portrait.thumbnail((512, 512), Image.Resampling.LANCZOS)
            if portrait.size != (512, 512):
                raise ValueError(f"{name} source must be square and at least 512 px")
            portrait.save(DESTINATION / f"{name}.png", optimize=True)


if __name__ == "__main__":
    main()
