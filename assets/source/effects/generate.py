"""Render the game's original UI and combat cues to Ogg Vorbis.

Run from the repository root: python3 assets/source/effects/generate.py
Uses Python's standard library and ffmpeg. A fixed seed makes the PCM source
repeatable; ffmpeg's native Vorbis encoder exports compact stereo Ogg files.
"""

from __future__ import annotations

import math
import random
import shutil
import struct
import subprocess
import tempfile
import wave
from pathlib import Path

RATE = 44_100
ROOT = Path(__file__).resolve().parents[3]
DEST = ROOT / "assets" / "effects" / "sounds"
TAU = math.tau


def envelope(time: float, duration: float, attack: float, release: float) -> float:
    return max(0.0, min(1.0, time / attack, (duration - time) / release))


def click(time: float, duration: float, noise: float) -> float:
    phase = TAU * (1050 * time - 520 * time * time / duration)
    body = math.sin(phase) + 0.23 * math.sin(phase * 2.01)
    return (body * math.exp(-time * 65) + noise * math.exp(-time * 210) * 0.08) * envelope(
        time, duration, 0.002, 0.012
    )


def laser(time: float, duration: float, noise: float) -> float:
    # A smooth downward chirp with a little electrical grit at the attack.
    phase = TAU * (
        1350 * time
        - 900 * time * time / duration
        + 300 * time**3 / duration**2
        + 130 * time
    )
    tone = 0.75 * math.sin(phase) + 0.17 * math.sin(2.04 * phase)
    return (tone + noise * 0.12 * math.exp(-time * 24)) * math.exp(-time * 8) * envelope(
        time, duration, 0.003, 0.045
    )


def shield(time: float, duration: float, noise: float) -> float:
    # Several close partials make a short, glassy resonant deflection.
    shimmer = (
        0.50 * math.sin(TAU * (510 * time + 170 * time * time))
        + 0.31 * math.sin(TAU * 760 * time)
        + 0.17 * math.sin(TAU * 1120 * time)
    )
    return (shimmer + noise * 0.08 * math.exp(-time * 24)) * math.exp(-time * 6) * envelope(
        time, duration, 0.004, 0.085
    )


def hull(time: float, duration: float, noise: float) -> float:
    # Inharmonic metal modes ring after a short impact transient.
    ring = (
        0.42 * math.sin(TAU * 340 * time)
        + 0.27 * math.sin(TAU * 653 * time)
        + 0.18 * math.sin(TAU * 1017 * time)
    ) * math.exp(-time * 17)
    crack = noise * math.exp(-time * 55) * 0.65
    return (ring + crack) * envelope(time, duration, 0.0015, 0.055)


def explosion(time: float, duration: float, noise: float) -> float:
    # Bass thump and decaying, filtered debris; no sampled source material.
    low = math.sin(TAU * (75 * time - 18 * time * time / duration)) * math.exp(-time * 11)
    debris = noise * math.exp(-time * 5.5)
    crackle = noise * math.sin(TAU * 32 * time) ** 8 * math.exp(-time * 9)
    return (0.58 * low + 0.55 * debris + 0.18 * crackle) * envelope(
        time, duration, 0.003, 0.14
    )


CUES = {
    "ui-click": (0.085, click, 0.0),
    "laser": (0.25, laser, 0.0),
    "shield": (0.36, shield, 0.12),
    "hull": (0.24, hull, 0.0),
    "explosion": (0.72, explosion, 0.20),
}


def samples(name: str, duration: float, synth, smoothing: float) -> list[float]:
    randomizer = random.Random(name)
    filtered = 0.0
    values = []
    for index in range(round(duration * RATE)):
        time = index / RATE
        white = randomizer.uniform(-1.0, 1.0)
        filtered = filtered * (1.0 - smoothing) + white * smoothing
        noise = filtered if smoothing else white
        values.append(synth(time, duration, noise))
    peak = max(abs(value) for value in values)
    return [0.82 * math.tanh(value * 1.4 / peak) / math.tanh(1.4) for value in values]


def export(name: str, duration: float, synth, smoothing: float) -> None:
    with tempfile.TemporaryDirectory() as directory:
        source = Path(directory) / f"{name}.wav"
        with wave.open(str(source), "wb") as output:
            output.setnchannels(1)
            output.setsampwidth(2)
            output.setframerate(RATE)
            output.writeframes(
                b"".join(
                    struct.pack("<h", round(value * 32767))
                    for value in samples(name, duration, synth, smoothing)
                )
            )
        subprocess.run(
            [
                "ffmpeg", "-y", "-nostdin", "-loglevel", "error", "-i", str(source),
                "-ac", "2", "-c:a", "vorbis", "-strict", "-2", "-b:a", "96k",
                "-fflags", "+bitexact", "-flags:a", "+bitexact",
                str(DEST / f"{name}.ogg"),
            ],
            check=True,
        )


def main() -> None:
    if not shutil.which("ffmpeg"):
        raise SystemExit("ffmpeg is required to export Ogg files")
    DEST.mkdir(parents=True, exist_ok=True)
    for name, (duration, synth, smoothing) in CUES.items():
        export(name, duration, synth, smoothing)
        print(f"wrote {DEST / (name + '.ogg')}")


if __name__ == "__main__":
    main()
