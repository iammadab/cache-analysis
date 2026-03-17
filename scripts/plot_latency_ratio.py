#!/usr/bin/env python3

import csv
from pathlib import Path

import matplotlib.pyplot as plt


INPUT_PATH = Path("results/latency_aggregate.csv")
OUTPUT_PATH = Path("results/latency_ratio_plot.png")


def read_aggregate(path: Path):
    sizes = []
    medians = []

    with path.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            sizes.append(int(row["size_bytes"]))
            medians.append(float(row["median_cpa"]))

    return sizes, medians


def adjacent_ratios(sizes, medians):
    x_values = []
    ratios = []

    for i in range(len(medians) - 1):
        if medians[i] == 0.0:
            continue
        x_values.append(sizes[i + 1])
        ratios.append(medians[i + 1] / medians[i])

    return x_values, ratios


def main():
    if not INPUT_PATH.exists():
        raise SystemExit(f"missing input file: {INPUT_PATH}")

    sizes, medians = read_aggregate(INPUT_PATH)
    if len(sizes) < 2:
        raise SystemExit(f"not enough points in input file: {INPUT_PATH}")

    x_values, ratios = adjacent_ratios(sizes, medians)
    if not x_values:
        raise SystemExit("no valid adjacent ratios computed")

    plt.figure(figsize=(10, 6))
    plt.plot(x_values, ratios, marker="o", linewidth=1.5)
    plt.axhline(1.0, color="gray", linestyle="--", linewidth=1.0)
    plt.xscale("log", base=2)
    plt.xlabel("Working set size (bytes)")
    plt.ylabel("Adjacent ratio (next/current)")
    plt.title("Latency Adjacent Ratios")
    plt.grid(True, which="both", linestyle="--", alpha=0.4)
    plt.tight_layout()

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(OUTPUT_PATH, dpi=150)
    print(f"wrote={OUTPUT_PATH}")


if __name__ == "__main__":
    main()
