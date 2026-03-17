use crate::types::AggregateRow;

#[derive(Clone, Copy, PartialEq, Eq)]
enum KneeClass {
    Minor,
    Major,
}

struct KneePoint {
    index: usize,
    ratio: f64,
    class: KneeClass,
}

struct KneeZone {
    from_size: u64,
    to_size: u64,
    peak_ratio: f64,
    class: KneeClass,
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

pub fn print_latency_inference_report(aggregate_rows: &[AggregateRow]) {
    if aggregate_rows.len() < 2 {
        println!("Latency Inference Report");
        println!("insufficient points for inference");
        return;
    }

    let mut ratios = Vec::with_capacity(aggregate_rows.len() - 1);
    for i in 0..(aggregate_rows.len() - 1) {
        let current = aggregate_rows[i].median_cpa;
        let next = aggregate_rows[i + 1].median_cpa;
        if current > 0.0 {
            ratios.push(next / current);
        }
    }

    if ratios.is_empty() {
        println!("Latency Inference Report");
        println!("no valid adjacent ratios for inference");
        return;
    }

    let p50 = nearest_rank_percentile(ratios.clone(), 50).expect("non-empty ratios");
    let p90 = nearest_rank_percentile(ratios.clone(), 90).expect("non-empty ratios");
    let p97 = nearest_rank_percentile(ratios.clone(), 97).expect("non-empty ratios");

    // Thresholds are data-driven from the current run's ratio distribution.
    // We use P90 as the minor knee floor and P97 as the major knee floor,
    // then enforce a minimum separation so major knees remain meaningfully stronger.
    let minor_threshold = p90.max(1.10);
    let major_threshold = p97.max(minor_threshold + 0.10);

    let mut knee_points = Vec::new();
    for i in 0..(aggregate_rows.len() - 1) {
        let current = aggregate_rows[i].median_cpa;
        if current <= 0.0 {
            continue;
        }

        let ratio = aggregate_rows[i + 1].median_cpa / current;
        if ratio >= major_threshold {
            knee_points.push(KneePoint {
                index: i,
                ratio,
                class: KneeClass::Major,
            });
        } else if ratio >= minor_threshold {
            knee_points.push(KneePoint {
                index: i,
                ratio,
                class: KneeClass::Minor,
            });
        }
    }

    let mut zones = Vec::<KneeZone>::new();
    for point in knee_points {
        if let Some(last) = zones.last_mut() {
            let last_to_index = aggregate_rows
                .iter()
                .position(|row| row.size_bytes == last.to_size)
                .unwrap_or(0)
                .saturating_sub(1);

            if point.index <= last_to_index + 1 {
                last.to_size = aggregate_rows[point.index + 1].size_bytes;
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
            from_size: aggregate_rows[point.index].size_bytes,
            to_size: aggregate_rows[point.index + 1].size_bytes,
            peak_ratio: point.ratio,
            class: point.class,
        });
    }

    println!("Latency Inference Report");
    println!("ratio_stats: P50={:.3} P90={:.3} P97={:.3}", p50, p90, p97);
    println!(
        "thresholds: minor={:.3} major={:.3}",
        minor_threshold, major_threshold
    );

    if zones.is_empty() {
        println!("knees: none detected");
        println!(
            "regions: single regime across {} to {}",
            fmt_size(aggregate_rows.first().expect("non-empty").size_bytes),
            fmt_size(aggregate_rows.last().expect("non-empty").size_bytes)
        );
    } else {
        println!("knees:");
        for zone in &zones {
            println!(
                "- {} -> {}, peak_ratio={:.3}, class={}",
                fmt_size(zone.from_size),
                fmt_size(zone.to_size),
                zone.peak_ratio,
                class_name(zone.class)
            );
        }

        let labels = ["L1-like", "L2-like", "LLC-like", "DRAM-like"];
        let region_boundaries: Vec<u64> = zones.iter().take(3).map(|zone| zone.to_size).collect();

        println!("regions:");
        let mut start = aggregate_rows.first().expect("non-empty").size_bytes;
        for (i, boundary) in region_boundaries.iter().enumerate() {
            let label = labels.get(i).copied().unwrap_or("post-knee-like");
            println!(
                "- {}: {} to {}",
                label,
                fmt_size(start),
                fmt_size(*boundary)
            );
            start = *boundary;
        }

        let final_label = labels
            .get(region_boundaries.len())
            .copied()
            .unwrap_or("post-knee-like");
        let end = aggregate_rows.last().expect("non-empty").size_bytes;
        println!(
            "- {}: {} to {}",
            final_label,
            fmt_size(start),
            fmt_size(end)
        );
    }

    println!(
        "caveat: inferred ranges are effective runtime boundaries, not exact hardware spec sizes"
    );
}
