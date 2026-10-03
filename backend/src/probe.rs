use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::{
    net::{lookup_host, TcpStream},
    time::{sleep, timeout},
};

const MAX_SAMPLES: u8 = 10;
const ALLOWED_PORTS: [u16; 2] = [80, 443];
const DNS_TIMEOUT: Duration = Duration::from_secs(3);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const SAMPLE_GAP: Duration = Duration::from_millis(100);

#[derive(Deserialize)]
pub struct ProbeRequest {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_samples")]
    pub samples: u8,
}

fn default_port() -> u16 {
    443
}

fn default_samples() -> u8 {
    5
}

#[derive(Serialize)]
pub struct ProbeResponse {
    pub host: String,
    pub port: u16,
    pub ip: IpAddr,
    pub dns_ms: f64,
    /// One entry per attempt, in order. `null` means the connection failed or timed out.
    pub samples_ms: Vec<Option<f64>>,
    /// `null` when every attempt failed.
    pub stats: Option<Stats>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Stats {
    pub min_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
}

/// A problem with the request itself; the message is safe to show to the caller.
#[derive(Debug)]
pub struct ProbeError(pub String);

/// Measures TCP connect time (roughly one network round trip) from this server to `host`.
pub async fn run(req: ProbeRequest) -> Result<ProbeResponse, ProbeError> {
    let host = normalize_host(&req.host)?;
    if !ALLOWED_PORTS.contains(&req.port) {
        return Err(ProbeError("Port must be 80 or 443".into()));
    }
    if req.samples == 0 || req.samples > MAX_SAMPLES {
        return Err(ProbeError(format!(
            "Samples must be between 1 and {MAX_SAMPLES}"
        )));
    }

    let dns_start = Instant::now();
    let addrs: Vec<SocketAddr> = timeout(DNS_TIMEOUT, lookup_host((host.as_str(), req.port)))
        .await
        .map_err(|_| ProbeError(format!("DNS lookup for {host} timed out")))?
        .map_err(|_| ProbeError(format!("Could not resolve {host}")))?
        .collect();
    let dns_ms = to_ms(dns_start.elapsed());

    // Connect to the address we just vetted rather than the hostname, so a second
    // DNS answer can't redirect the probe to a private address.
    let target = pick_target(&addrs)
        .ok_or_else(|| ProbeError(format!("{host} does not resolve to a public address")))?;

    let mut samples_ms = Vec::with_capacity(req.samples as usize);
    for i in 0..req.samples {
        if i > 0 {
            sleep(SAMPLE_GAP).await;
        }
        let start = Instant::now();
        let connected = matches!(
            timeout(CONNECT_TIMEOUT, TcpStream::connect(target)).await,
            Ok(Ok(_))
        );
        samples_ms.push(connected.then(|| to_ms(start.elapsed())));
    }

    let successes: Vec<f64> = samples_ms.iter().flatten().copied().collect();

    Ok(ProbeResponse {
        host,
        port: req.port,
        ip: target.ip(),
        dns_ms,
        samples_ms,
        stats: stats(&successes),
    })
}

fn normalize_host(input: &str) -> Result<String, ProbeError> {
    let host = input.trim().to_ascii_lowercase();
    if host.is_empty() || host.len() > 253 {
        return Err(ProbeError("Enter a hostname like example.com".into()));
    }
    if host.parse::<IpAddr>().is_ok() {
        return Ok(host);
    }

    let valid_label = |label: &str| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    if !host.split('.').all(valid_label) {
        return Err(ProbeError(
            "Enter just a hostname like example.com (no https:// or path)".into(),
        ));
    }
    Ok(host)
}

/// First public address, preferring IPv4 since many hosts (and Docker by default) lack IPv6.
fn pick_target(addrs: &[SocketAddr]) -> Option<SocketAddr> {
    addrs
        .iter()
        .filter(|addr| is_public(addr.ip()))
        .min_by_key(|addr| addr.is_ipv6())
        .copied()
}

/// Rejects loopback, private, link-local and other non-routable ranges so the probe
/// can't be pointed at the server's own network.
fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_v4(v4),
            None => is_public_v6(v6),
        },
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        || ip.is_multicast()
        || a == 0
        || (a == 100 && (b & 0xc0) == 64) // 100.64.0.0/10 carrier-grade NAT
        || (a == 192 && b == 0 && c == 0) // 192.0.0.0/24 protocol assignments
        || (a == 198 && (b & 0xfe) == 18) // 198.18.0.0/15 benchmarking
        || a >= 240) // 240.0.0.0/4 reserved
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let seg = ip.segments();
    !(ip.is_multicast()
        || seg[..6] == [0; 6] // ::/96 covers unspecified, loopback, IPv4-compatible
        || (seg[0] & 0xfe00) == 0xfc00 // fc00::/7 unique local
        || (seg[0] & 0xffc0) == 0xfe80 // fe80::/10 link-local
        || (seg[0] == 0x2001 && seg[1] == 0x0db8) // 2001:db8::/32 documentation
        || (seg[0] == 0x0064 && seg[1] == 0xff9b)) // 64:ff9b::/32 NAT64, can embed private IPv4
}

fn stats(successes: &[f64]) -> Option<Stats> {
    if successes.is_empty() {
        return None;
    }
    let mut sorted = successes.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));

    let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
    Some(Stats {
        min_ms: sorted[0],
        avg_ms: round2(avg),
        p50_ms: percentile(&sorted, 50.0),
        p95_ms: percentile(&sorted, 95.0),
        max_ms: sorted[sorted.len() - 1],
    })
}

/// Nearest-rank percentile of an ascending, non-empty slice.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = (p / 100.0 * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn to_ms(d: Duration) -> f64 {
    round2(d.as_secs_f64() * 1000.0)
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn public_addresses_are_allowed() {
        for addr in ["1.1.1.1", "8.8.8.8", "140.82.121.4", "2606:4700:4700::1111"] {
            assert!(is_public(ip(addr)), "{addr} should be public");
        }
    }

    #[test]
    fn internal_addresses_are_rejected() {
        for addr in [
            "127.0.0.1",
            "10.0.0.5",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "255.255.255.255",
            "::1",
            "::",
            "fe80::1",
            "fd00::1",
            "::ffff:10.0.0.1",
            "::ffff:127.0.0.1",
            "64:ff9b::a00:1",
        ] {
            assert!(!is_public(ip(addr)), "{addr} should be rejected");
        }
    }

    #[test]
    fn pick_target_prefers_public_ipv4() {
        let addrs: Vec<SocketAddr> = vec![
            "[2606:4700:4700::1111]:443".parse().unwrap(),
            "10.0.0.1:443".parse().unwrap(),
            "1.1.1.1:443".parse().unwrap(),
        ];
        assert_eq!(pick_target(&addrs), Some("1.1.1.1:443".parse().unwrap()));
    }

    #[test]
    fn pick_target_is_none_when_everything_is_private() {
        let addrs: Vec<SocketAddr> = vec!["127.0.0.1:443".parse().unwrap()];
        assert_eq!(pick_target(&addrs), None);
    }

    #[test]
    fn normalize_host_accepts_hostnames_and_ips() {
        assert_eq!(normalize_host("  Example.COM ").unwrap(), "example.com");
        assert_eq!(normalize_host("1.1.1.1").unwrap(), "1.1.1.1");
        assert_eq!(normalize_host("sub-domain.example.co").unwrap(), "sub-domain.example.co");
    }

    #[test]
    fn normalize_host_rejects_urls_and_junk() {
        for bad in ["", "https://example.com", "example.com/path", "exa mple.com", "-bad.com", "a..b", "example.com:22"] {
            assert!(normalize_host(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn stats_of_known_samples() {
        let got = stats(&[30.0, 10.0, 20.0, 40.0]).unwrap();
        assert_eq!(
            got,
            Stats {
                min_ms: 10.0,
                avg_ms: 25.0,
                p50_ms: 20.0,
                p95_ms: 40.0,
                max_ms: 40.0,
            }
        );
    }

    #[test]
    fn stats_of_nothing_is_none() {
        assert_eq!(stats(&[]), None);
    }
}
