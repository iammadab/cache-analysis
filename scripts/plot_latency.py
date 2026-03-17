#!/usr/bin/env python3

import csv
from pathlib import Path

import matplotlib.pyplot as plt


INPUT_PATH = Path("results/latency_aggregate.csv")
OUTPUT_PATH = Path("results/latency_plot.png")
ZOOM_OUTPUT_PATH = Path("results/latency_plot_zoom_0_50.png")


def read_aggregate(path: Path):
    sizes = []
    medians = []

    with path.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            sizes.append(int(row["size_bytes"]))
            medians.append(float(row["median_cpa"]))

    return sizes, medians


def main():
    if not INPUT_PATH.exists():
        raise SystemExit(f"missing input file: {INPUT_PATH}")

    sizes, medians = read_aggregate(INPUT_PATH)
    if not sizes:
        raise SystemExit(f"no data in input file: {INPUT_PATH}")

    plt.figure(figsize=(10, 6))
    plt.plot(sizes, medians, marker="o", linewidth=1.5)
    plt.xscale("log", base=2)
    plt.xlabel("Working set size (bytes)")
    plt.ylabel("Median cycles/access")
    plt.title("Latency Ladder")
    plt.grid(True, which="both", linestyle="--", alpha=0.4)
    plt.tight_layout()

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(OUTPUT_PATH, dpi=150)
    plt.close()

    plt.figure(figsize=(10, 6))
    plt.plot(sizes, medians, marker="o", linewidth=1.5)
    plt.xscale("log", base=2)
    plt.ylim(0, 50)
    plt.xlabel("Working set size (bytes)")
    plt.ylabel("Median cycles/access")
    plt.title("Latency Ladder (Zoomed: 0-50 cycles/access)")
    plt.grid(True, which="both", linestyle="--", alpha=0.4)
    plt.tight_layout()
    plt.savefig(ZOOM_OUTPUT_PATH, dpi=150)
    plt.close()

    print(f"wrote={OUTPUT_PATH}")
    print(f"wrote={ZOOM_OUTPUT_PATH}")


if __name__ == "__main__":
    main()
