use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Entries are pruned once the map grows past this, so memory stays bounded.
const PRUNE_THRESHOLD: usize = 10_000;

/// Fixed-window limiter: each IP gets `limit` requests per `window`.
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    hits: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Records a request from `ip` and returns whether it is allowed.
    pub fn check(&self, ip: IpAddr) -> bool {
        self.check_at(ip, Instant::now())
    }

    fn check_at(&self, ip: IpAddr, now: Instant) -> bool {
        let mut hits = self.hits.lock().unwrap();

        if hits.len() > PRUNE_THRESHOLD {
            hits.retain(|_, (start, _)| now.duration_since(*start) < self.window);
        }

        let entry = hits.entry(ip).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_limit_and_resets_after_window() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip: IpAddr = "203.0.113.7".parse().unwrap();
        let t0 = Instant::now();

        assert!(limiter.check_at(ip, t0));
        assert!(limiter.check_at(ip, t0));
        assert!(!limiter.check_at(ip, t0));
        assert!(limiter.check_at(ip, t0 + Duration::from_secs(61)));
    }

    #[test]
    fn tracks_each_ip_separately() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let t0 = Instant::now();

        assert!(limiter.check_at("203.0.113.7".parse().unwrap(), t0));
        assert!(limiter.check_at("203.0.113.8".parse().unwrap(), t0));
    }
}
