use crate::types::AggregateRow;

#[derive(Clone, Copy, PartialEq, Eq)]
enum KneeClass {
    Minor,
    Major,
}

#[derive(Clone, Copy)]
struct KneePoint {
    index: usize,
    ratio: f64,
    class: KneeClass,
}

#[derive(Clone, Copy)]
struct KneeZone {
    start_index: usize,
    end_index: usize,
    peak_ratio: f64,
    class: KneeClass,
}

#[derive(Clone, Copy)]
struct Thresholds {
    p50: f64,
    p90: f64,
    p97: f64,
    minor: f64,
    major: f64,
}

#[derive(Clone, Copy)]
struct Region {
    label: &'static str,
    from_size: u64,
    to_size: u64,
}

struct InferenceResult {
    thresholds: Thresholds,
    zones: Vec<KneeZone>,
    regions: Vec<Region>,
}

fn nearest_rank_percentile(mut values: Vec<f64>, percentile: u32) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).expect("no NaN values expected"));
    let n = values.len();
    let rank = ((percentile as usize * n) + 99) / 100;
    let idx = rank.saturating_sub(1).min(n - 1);
    Some(values[idx])
}

fn class_name(class: KneeClass) -> &'static str {
    match class {
        KneeClass::Minor => "minor",
        KneeClass::Major => "major",
    }
}

// Formats byte sizes for readable report output.
fn fmt_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / KIB)
    } else {
        format!("{:.1} MiB", bytes as f64 / MIB)
    }
}

// Computes adjacent latency ratios (next/current) and keeps the left index.
fn compute_adjacent_ratios(rows: &[AggregateRow]) -> Vec<(usize, f64)> {
    let mut ratios = Vec::with_capacity(rows.len().saturating_sub(1));
    for i in 0..rows.len().saturating_sub(1) {
        let current = rows[i].median_cpa;
        if current > 0.0 {
            ratios.push((i, rows[i + 1].median_cpa / current));
        }
    }
    ratios
}

// Derives data-driven knee thresholds from the ratio distribution.
fn compute_thresholds(ratios: &[f64]) -> Option<Thresholds> {
    if ratios.is_empty() {
        return None;
    }

    let p50 = nearest_rank_percentile(ratios.to_vec(), 50)?;
    let p90 = nearest_rank_percentile(ratios.to_vec(), 90)?;
    let p97 = nearest_rank_percentile(ratios.to_vec(), 97)?;

    // Thresholds are data-driven from the current run's ratio distribution.
    // We use P90 as the minor knee floor and P97 as the major knee floor,
    // then enforce a minimum separation so major knees remain meaningfully stronger.
    let minor = p90.max(1.10);
    let major = p97.max(minor + 0.10);

    Some(Thresholds {
        p50,
        p90,
        p97,
        minor,
        major,
    })
}

// Classifies each adjacent ratio as minor/major knee candidate.
fn classify_knee_points(adjacents: &[(usize, f64)], thresholds: Thresholds) -> Vec<KneePoint> {
    let mut points = Vec::new();
    for (index, ratio) in adjacents {
        if *ratio >= thresholds.major {
            points.push(KneePoint {
                index: *index,
                ratio: *ratio,
                class: KneeClass::Major,
            });
        } else if *ratio >= thresholds.minor {
            points.push(KneePoint {
                index: *index,
                ratio: *ratio,
                class: KneeClass::Minor,
            });
        }
    }
    points
}

// Merges neighboring knee points into wider transition zones.
fn merge_knee_points(points: &[KneePoint]) -> Vec<KneeZone> {
    let mut zones = Vec::<KneeZone>::new();

    for point in points {
        if let Some(last) = zones.last_mut() {
            if point.index <= last.end_index + 1 {
                last.end_index = point.index;
                if point.ratio > last.peak_ratio {
                    last.peak_ratio = point.ratio;
                }
                if point.class == KneeClass::Major {
                    last.class = KneeClass::Major;
                }
                continue;
            }
        }

        zones.push(KneeZone {
            start_index: point.index,
            end_index: point.index,
            peak_ratio: point.ratio,
            class: point.class,
        });
    }

    zones
}

// Converts knee zones into contiguous hierarchy-like regions.
fn infer_regions(rows: &[AggregateRow], zones: &[KneeZone]) -> Vec<Region> {
    let labels = ["L1-like", "L2-like", "LLC-like", "DRAM-like"];
    let mut boundaries = Vec::with_capacity(3);
    for zone in zones.iter().take(3) {
        boundaries.push(rows[(zone.end_index + 1).min(rows.len() - 1)].size_bytes);
    }

    let mut regions = Vec::new();
    let mut start = rows.first().expect("non-empty").size_bytes;

    for (i, boundary) in boundaries.iter().enumerate() {
        regions.push(Region {
            label: labels.get(i).copied().unwrap_or("post-knee-like"),
            from_size: start,
            to_size: *boundary,
        });
        start = *boundary;
    }

    regions.push(Region {
        label: labels
            .get(boundaries.len())
            .copied()
            .unwrap_or("post-knee-like"),
        from_size: start,
        to_size: rows.last().expect("non-empty").size_bytes,
    });

    regions
}

// Runs the full inference pipeline and returns structured results.
fn build_inference(rows: &[AggregateRow]) -> Option<InferenceResult> {
    let adjacents = compute_adjacent_ratios(rows);
    let ratio_values: Vec<f64> = adjacents.iter().map(|(_, ratio)| *ratio).collect();
    let thresholds = compute_thresholds(&ratio_values)?;

    let points = classify_knee_points(&adjacents, thresholds);
    let zones = merge_knee_points(&points);
    let regions = if zones.is_empty() {
        Vec::new()
    } else {
        infer_regions(rows, &zones)
    };

    Some(InferenceResult {
        thresholds,
        zones,
        regions,
    })
}

// Prints a human-readable inference report from aggregate latency data.
pub fn print_latency_inference_report(aggregate_rows: &[AggregateRow]) {
    println!("Latency Inference Report");

    if aggregate_rows.len() < 2 {
        println!("insufficient points for inference");
        return;
    }

    let Some(inference) = build_inference(aggregate_rows) else {
        println!("no valid adjacent ratios for inference");
        return;
    };

    println!(
        "ratio_stats: P50={:.3} P90={:.3} P97={:.3}",
        inference.thresholds.p50, inference.thresholds.p90, inference.thresholds.p97
    );
    println!(
        "thresholds: minor={:.3} major={:.3}",
        inference.thresholds.minor, inference.thresholds.major
    );

    if inference.zones.is_empty() {
        println!("knees: none detected");
        println!(
            "regions: single regime across {} to {}",
            fmt_size(aggregate_rows.first().expect("non-empty").size_bytes),
            fmt_size(aggregate_rows.last().expect("non-empty").size_bytes)
        );
    } else {
        println!("knees:");
        for zone in &inference.zones {
            let from_size = aggregate_rows[zone.start_index].size_bytes;
            let to_size =
                aggregate_rows[(zone.end_index + 1).min(aggregate_rows.len() - 1)].size_bytes;
            println!(
                "- {} -> {}, peak_ratio={:.3}, class={}",
                fmt_size(from_size),
                fmt_size(to_size),
                zone.peak_ratio,
                class_name(zone.class)
            );
        }

        println!("regions:");
        for region in &inference.regions {
            println!(
                "- {}: {} to {}",
                region.label,
                fmt_size(region.from_size),
                fmt_size(region.to_size)
            );
        }
    }

    println!(
        "caveat: inferred ranges are effective runtime boundaries, not exact hardware spec sizes"
    );
}
