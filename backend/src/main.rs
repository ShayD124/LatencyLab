mod probe;
mod rate_limit;

use std::{
    env,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{rejection::JsonRejection, ConnectInfo, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tokio::{net::TcpListener, sync::Semaphore};
use tower_http::cors::{AllowOrigin, CorsLayer};

use probe::{ProbeRequest, ProbeResponse};
use rate_limit::RateLimiter;

const PROBES_PER_MINUTE: u32 = 10;
const MAX_CONCURRENT_PROBES: usize = 8;

#[derive(Clone)]
struct AppState {
    limiter: Arc<RateLimiter>,
    probe_slots: Arc<Semaphore>,
    /// Only read X-Forwarded-For when a reverse proxy we control sits in front.
    trust_proxy: bool,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

type ApiError = (StatusCode, Json<ErrorBody>);

fn api_error(status: StatusCode, message: impl Into<String>) -> ApiError {
    (
        status,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port: u16 = env::var("PORT")
        .ok()
        .map(|p| p.parse().expect("PORT must be a number"))
        .unwrap_or(8080);
    let origins = allowed_origins();
    let state = AppState {
        limiter: Arc::new(RateLimiter::new(
            PROBES_PER_MINUTE,
            Duration::from_secs(60),
        )),
        probe_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_PROBES)),
        trust_proxy: env::var("TRUST_PROXY").is_ok_and(|v| v == "true"),
    };

    tracing::info!(?origins, "allowed CORS origins");
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/ping", get(ping))
        .route("/api/probe", post(run_probe))
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind port");
    tracing::info!("listening on port {port}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");
}

/// Reads the comma-separated `ALLOWED_ORIGINS`, e.g. `https://example.com,https://www.example.com`.
fn allowed_origins() -> Vec<HeaderValue> {
    env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".into())
        .split(',')
        .map(|origin| origin.trim().trim_end_matches('/'))
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            origin
                .parse()
                .unwrap_or_else(|_| panic!("invalid origin in ALLOWED_ORIGINS: {origin}"))
        })
        .collect()
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Serialize)]
struct Ping {
    server_time_ms: u128,
}

/// Tiny uncached response the browser times to measure its round trip to this server.
async fn ping() -> impl IntoResponse {
    let server_time_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(Ping { server_time_ms }),
    )
}

async fn run_probe(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<ProbeRequest>, JsonRejection>,
) -> Result<Json<ProbeResponse>, ApiError> {
    let Json(req) = body.map_err(|_| {
        api_error(
            StatusCode::BAD_REQUEST,
            r#"Request body must be JSON like {"host": "example.com"}"#,
        )
    })?;

    let client = client_ip(&headers, peer, state.trust_proxy);
    if !state.limiter.check(client) {
        return Err(api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many probes. Try again in a minute.",
        ));
    }
    let _slot = state.probe_slots.try_acquire().map_err(|_| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Server is busy. Try again shortly.",
        )
    })?;

    tracing::info!(%client, host = %req.host, port = req.port, samples = req.samples, "probe");
    probe::run(req)
        .await
        .map(Json)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, e.0))
}

/// The caller's address: the entry our own proxy appended to X-Forwarded-For when
/// `trust_proxy` is on, otherwise the socket peer.
fn client_ip(headers: &HeaderMap, peer: SocketAddr, trust_proxy: bool) -> IpAddr {
    if !trust_proxy {
        return peer.ip();
    }
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .and_then(|last| last.trim().parse().ok())
        .unwrap_or_else(|| peer.ip())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
    };

    // `docker stop` sends SIGTERM.
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> SocketAddr {
        "172.18.0.2:5000".parse().unwrap()
    }

    #[test]
    fn client_ip_ignores_forwarded_header_without_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
        assert_eq!(client_ip(&headers, peer(), false), peer().ip());
    }

    #[test]
    fn client_ip_uses_last_forwarded_entry_behind_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "6.6.6.6, 203.0.113.9".parse().unwrap());
        let expected: IpAddr = "203.0.113.9".parse().unwrap();
        assert_eq!(client_ip(&headers, peer(), true), expected);
    }

    #[test]
    fn client_ip_falls_back_to_peer_when_header_missing() {
        assert_eq!(client_ip(&HeaderMap::new(), peer(), true), peer().ip());
    }
}
