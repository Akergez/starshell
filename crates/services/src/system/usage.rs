use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UsageStatus {
    pub cpu: Option<u8>,
    pub gpu: Option<u8>,
    pub memory_used_kib: u64,
    pub memory_total_kib: u64,
}

fn cpu_times(stat: &str) -> Option<(u64, u64)> {
    let mut fields = stat.lines().next()?.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let values = fields
        .take(8)
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if values.len() < 5 {
        return None;
    }
    Some((values.iter().sum(), values[3] + values[4]))
}

fn cpu_percent(previous: (u64, u64), current: (u64, u64)) -> Option<u8> {
    let total = current.0.checked_sub(previous.0)?;
    let idle = current.1.checked_sub(previous.1)?;
    if total == 0 {
        return None;
    }
    Some(((total.saturating_sub(idle) as f64 / total as f64 * 100.0).round() as u8).min(100))
}

fn memory_usage(meminfo: &str) -> (u64, u64) {
    let value = |key: &str| {
        meminfo.lines().find_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next()? == key)
                .then(|| fields.next()?.parse::<u64>().ok())
                .flatten()
        })
    };
    match (value("MemTotal:"), value("MemAvailable:")) {
        (Some(total), Some(available)) => (total.saturating_sub(available), total),
        _ => (0, 0),
    }
}

async fn gpu_percent() -> Option<u8> {
    if let Ok(mut entries) = tokio::fs::read_dir("/sys/class/drm").await {
        let mut maximum = None;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.strip_prefix("card").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
            }) {
                continue;
            }
            if let Ok(value) =
                tokio::fs::read_to_string(entry.path().join("device/gpu_busy_percent")).await
                && let Ok(percent) = value.trim().parse::<u8>()
            {
                maximum = Some(maximum.unwrap_or(0).max(percent.min(100)));
            }
        }
        if maximum.is_some() {
            return maximum;
        }
    }
    let output = tokio::time::timeout(
        Duration::from_millis(800),
        tokio::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u8>().ok())
        .map(|value| value.min(100))
        .max()
}

pub(super) async fn sample(previous: &mut Option<(u64, u64)>) -> UsageStatus {
    let (stat, memory, gpu) = tokio::join!(
        tokio::fs::read_to_string("/proc/stat"),
        tokio::fs::read_to_string("/proc/meminfo"),
        gpu_percent()
    );
    let current = stat.ok().and_then(|stat| cpu_times(&stat));
    let cpu = previous
        .zip(current)
        .and_then(|(previous, current)| cpu_percent(previous, current));
    *previous = current;
    let (memory_used_kib, memory_total_kib) =
        memory.map(|value| memory_usage(&value)).unwrap_or_default();
    UsageStatus {
        cpu,
        gpu,
        memory_used_kib,
        memory_total_kib,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_excludes_guest_ticks_and_uses_interval_deltas() {
        let previous = cpu_times("cpu 100 20 30 400 50 0 0 0 20 10\n").expect("cpu counters");
        let current = cpu_times("cpu 130 20 50 450 50 0 0 0 30 10\n").expect("cpu counters");
        assert_eq!(cpu_percent(previous, current), Some(50));
        assert_eq!(cpu_percent(current, current), None);
        assert_eq!(cpu_percent(current, previous), None);
    }

    #[test]
    fn memory_uses_available_including_reclaimable_cache() {
        assert_eq!(memory_usage("MemTotal: 16000 kB\n"), (0, 0));
        assert_eq!(
            memory_usage("MemTotal: 16000 kB\nMemFree: 1000 kB\nMemAvailable: 6000 kB\n"),
            (10000, 16000)
        );
    }
}
