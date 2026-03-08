use super::model::{BenchResult, CumulativeBucket};

impl BenchResult {
    pub fn latency_for_percentile(&self, percentile: f64) -> f64 {
        percentile_ms(&self.samples_ns, percentile)
    }

    pub fn cumulative_count_for_percentile(&self, percentile: f64) -> u64 {
        if self.samples_ns.is_empty() {
            return 0;
        }
        if percentile <= 0.0 {
            return 1;
        }
        let index = percentile_index(self.samples_ns.len(), percentile);
        (index + 1) as u64
    }
}

pub fn build_cumulative_distribution(samples: &[u64]) -> Vec<CumulativeBucket> {
    if samples.is_empty() {
        return Vec::new();
    }

    let max_ms = ns_to_ms(*samples.last().unwrap_or(&0));
    let step = rounded_threshold(max_ms / 8.0).max(0.001);
    let mut buckets = Vec::new();
    let mut threshold = step;

    while threshold < max_ms {
        let count = samples.partition_point(|value| ns_to_ms(*value) <= threshold) as u64;
        if count > 0 {
            buckets.push(CumulativeBucket {
                percent: count as f64 * 100.0 / samples.len() as f64,
                latency_ms: threshold,
                cumulative_count: count,
            });
        }
        threshold += step;
    }

    buckets.push(CumulativeBucket {
        percent: 100.0,
        latency_ms: rounded_threshold(max_ms + step),
        cumulative_count: samples.len() as u64,
    });
    buckets
}

pub fn percentile_ms(samples_ns: &[u64], percentile: f64) -> f64 {
    if samples_ns.is_empty() {
        return 0.0;
    }
    ns_to_ms(samples_ns[percentile_index(samples_ns.len(), percentile)])
}

pub fn percentile_index(len: usize, percentile: f64) -> usize {
    if len <= 1 || percentile <= 0.0 {
        return 0;
    }
    let rank = ((percentile / 100.0) * (len.saturating_sub(1)) as f64).round() as usize;
    rank.min(len.saturating_sub(1))
}

pub fn ns_to_ms(value: u64) -> f64 {
    value as f64 / 1_000_000.0
}

fn rounded_threshold(value: f64) -> f64 {
    if value <= 0.001 {
        0.001
    } else {
        (value * 1000.0).ceil() / 1000.0
    }
}
