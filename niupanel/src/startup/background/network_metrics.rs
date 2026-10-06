use std::collections::HashMap;
use std::time::Instant;

#[derive(Default)]
pub(super) struct NetworkSampler {
    previous: HashMap<String, (u64, u64)>,
    sampled_at: Option<Instant>,
}

impl NetworkSampler {
    pub(super) fn sample<'a>(
        &mut self,
        interfaces: impl IntoIterator<Item = (&'a str, u64, u64)>,
        now: Instant,
    ) -> (Option<f64>, Option<f64>) {
        let mut current = HashMap::new();
        let mut uploaded = 0.0;
        let mut downloaded = 0.0;
        for (name, sent, received) in interfaces {
            if name == "lo"
                || name.strip_prefix("lo").is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                })
                || name.starts_with("Loopback")
            {
                continue;
            }
            // New interfaces establish a baseline; counter resets must not produce spikes.
            if let Some(&(previous_sent, previous_received)) = self.previous.get(name) {
                uploaded += sent.saturating_sub(previous_sent) as f64;
                downloaded += received.saturating_sub(previous_received) as f64;
            }
            current.insert(name.to_owned(), (sent, received));
        }

        let elapsed = self.sampled_at.map(|previous| now.duration_since(previous));
        self.previous = current;
        self.sampled_at = Some(now);
        let Some(elapsed) = elapsed.filter(|elapsed| !elapsed.is_zero()) else {
            return (None, None);
        };

        (
            Some(uploaded / elapsed.as_secs_f64()),
            Some(downloaded / elapsed.as_secs_f64()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkSampler;
    use std::time::{Duration, Instant};

    #[test]
    fn measures_direction_and_actual_elapsed_time_without_loopback() {
        let mut sampler = NetworkSampler::default();
        let now = Instant::now();
        assert_eq!(
            sampler.sample([("eth0", 10_000, 20_000), ("lo", 100, 100)], now),
            (None, None)
        );
        assert_eq!(
            sampler.sample(
                [("eth0", 10_750, 23_000), ("lo", 100_000, 100_000)],
                now + Duration::from_millis(1500),
            ),
            (Some(500.0), Some(2000.0))
        );
    }

    #[test]
    fn handles_resets_new_interfaces_and_reappearing_interfaces() {
        let mut sampler = NetworkSampler::default();
        let now = Instant::now();
        sampler.sample([("eth0", 10_000, 20_000)], now);
        assert_eq!(
            sampler.sample(
                [("eth0", 5, 10), ("eth1", 1_000_000, 1_000_000)],
                now + Duration::from_secs(3),
            ),
            (Some(0.0), Some(0.0))
        );
        sampler.sample([], now + Duration::from_secs(6));
        assert_eq!(
            sampler.sample([("eth0", 50_000, 50_000)], now + Duration::from_secs(9)),
            (Some(0.0), Some(0.0))
        );
    }

    #[test]
    fn aggregates_interfaces_and_handles_zero_duration() {
        let mut sampler = NetworkSampler::default();
        let now = Instant::now();
        sampler.sample([("eth0", 0, 0), ("eth1", 0, 0)], now);
        assert_eq!(
            sampler.sample([("eth0", 0, 0), ("eth1", 0, 0)], now),
            (None, None)
        );
        assert_eq!(
            sampler.sample(
                [("eth0", 300, 600), ("eth1", 600, 1200)],
                now + Duration::from_secs(3),
            ),
            (Some(300.0), Some(600.0))
        );
    }
}
