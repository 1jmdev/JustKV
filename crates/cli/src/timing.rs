use std::time::Duration;

pub fn render_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    format!(
        "(time) {nanos} ns ({}, {})",
        format_scaled(nanos, 1_000, "us", 3),
        format_scaled(nanos, 1_000_000, "ms", 6),
    )
}

fn format_scaled(nanos: u128, divisor: u128, suffix: &str, digits: usize) -> String {
    let whole = nanos / divisor;
    let fraction = nanos % divisor;
    let scale = 10u128.pow(digits as u32);
    let decimal = (fraction * scale) / divisor;
    format!("{whole}.{decimal:0digits$} {suffix}")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::render_duration;

    #[test]
    fn renders_precise_units() {
        assert_eq!(
            render_duration(Duration::from_nanos(1_234_567)),
            "(time) 1234567 ns (1234.567 us, 1.234567 ms)"
        );
    }
}
