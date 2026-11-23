"""Copy the selected, unmodified CC0 Kenney assets from official ZIP archives.

Usage: python3 tools/art/import_kenney_effects.py SCI_FI_ZIP UI_ZIP PARTICLE_ZIP
The archives are downloaded from the asset pages listed in assets/effects/README.md.
"""

from pathlib import Path
from sys import argv
from zipfile import ZipFile


ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "assets" / "effects"
PACKS = (
    ("sci-fi-sounds", "sounds", (
        "Audio/laserLarge_000.ogg",
        "Audio/forceField_000.ogg",
        "Audio/impactMetal_001.ogg",
        "Audio/explosionCrunch_000.ogg",
    )),
    ("ui-audio", "sounds", ("Audio/click3.ogg",)),
    ("particle-pack", "particles", (
        "PNG (Transparent)/circle_03.png",
        "PNG (Transparent)/spark_01.png",
        "PNG (Transparent)/smoke_03.png",
    )),
)


def main() -> None:
    if len(argv) != 4:
        raise SystemExit(__doc__)
    for (pack, kind, names), archive in zip(PACKS, argv[1:], strict=True):
        with ZipFile(archive) as source:
            license_text = source.read("License.txt")
            if b"CC0" not in license_text:
                raise ValueError(f"{archive} is missing the expected CC0 license")
            license_path = ASSETS / "licenses" / f"{pack}.txt"
            license_path.parent.mkdir(parents=True, exist_ok=True)
            license_path.write_bytes(license_text)
            for name in names:
                destination = ASSETS / kind / Path(name).name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(source.read(name))


if __name__ == "__main__":
    main()
