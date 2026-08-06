//! Local Axum API for the dashboard snapshot and Server-Sent Events feed.

use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_stream::stream;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    net::TcpListener,
    time::{MissedTickBehavior, interval},
};

use crate::{
    domain::{Client, SnapshotV1},
    error::{AppError, Result},
};

pub(crate) const DEFAULT_HOST: &str = "127.0.0.1";
pub(crate) const DEFAULT_PORT: u16 = 4477;
pub(crate) const CACHE_SECONDS: u64 = 2;
pub(crate) const HEARTBEAT_SECONDS: u64 = 20;
pub(crate) const POLL_SECONDS: u64 = 1;

/// Dashboard-only message row.  The status schema deliberately excludes these
/// fields; they are added only to snapshots served over HTTP/SSE.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct SnapshotMessageV1 {
    pub(crate) id: String,
    pub(crate) sender_client: Client,
    pub(crate) sender_session_id: String,
    pub(crate) sender_callsign: Option<String>,
    pub(crate) recipient_client: Client,
    pub(crate) recipient_session_id: String,
    pub(crate) recipient_callsign: Option<String>,
    pub(crate) repo_root: Option<String>,
    pub(crate) text: String,
    pub(crate) created_at: f64,
    pub(crate) acknowledged_at: Option<f64>,
}

/// The dashboard's complete runtime shape, including its server metadata.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct DashboardSnapshotV1 {
    #[serde(flatten)]
    pub(crate) snapshot: SnapshotV1,
    pub(crate) messages: Vec<SnapshotMessageV1>,
    pub(crate) generated_at: String,
    pub(crate) generation: u64,
}

/// Minimal bridge from coordination/state code to this transport layer.
///
/// `generation` intentionally is not cached: the event loop calls it every
/// second so its implementation can cheaply reconcile process state and bump
/// the counter before deciding whether clients need a new snapshot.
pub(crate) trait SnapshotSource: Send + Sync + 'static {
    fn snapshot(&self) -> Result<SnapshotV1>;
    fn messages(&self) -> Result<Vec<SnapshotMessageV1>>;
    fn generation(&self) -> Result<u64>;
}

pub(crate) struct SnapshotService<S> {
    source: Arc<S>,
    cache_ttl: Duration,
    cache: Mutex<Option<CachedSnapshot>>,
    now: Arc<dyn Fn() -> SystemTime + Send + Sync>,
}

struct CachedSnapshot {
    refreshed_at: Instant,
    payload: DashboardSnapshotV1,
}

impl<S: SnapshotSource> SnapshotService<S> {
    pub(crate) fn new(source: S) -> Self {
        Self::with_cache_ttl(source, Duration::from_secs(CACHE_SECONDS))
    }

    pub(crate) fn with_cache_ttl(source: S, cache_ttl: Duration) -> Self {
        Self { source: Arc::new(source), cache_ttl, cache: Mutex::new(None), now: Arc::new(SystemTime::now) }
    }

    /// Return the process-wide cached snapshot, refreshing at most once per two
    /// seconds in the normal constructor.
    pub(crate) fn snapshot(&self) -> Result<DashboardSnapshotV1> {
        let mut cache = self.cache.lock().expect("snapshot cache lock poisoned");
        if let Some(cached) = cache.as_ref() &&
            cached.refreshed_at.elapsed() < self.cache_ttl
        {
            return Ok(cached.payload.clone());
        }

        let payload = DashboardSnapshotV1 {
            snapshot: self.source.snapshot()?,
            messages: self.source.messages()?,
            generated_at: rfc3339_utc((self.now)()),
            generation: self.source.generation()?,
        };
        *cache = Some(CachedSnapshot { refreshed_at: Instant::now(), payload: payload.clone() });
        Ok(payload)
    }

    /// Deliberately bypass the snapshot cache; see [`SnapshotSource`].
    pub(crate) fn generation(&self) -> Result<u64> {
        self.source.generation()
    }
}

/// Construct the dashboard API.  It is separate from binding so callers can
/// embed it in their own runtime or tests.
pub(crate) fn router<S: SnapshotSource>(service: SnapshotService<S>) -> Router {
    let state = Arc::new(service);
    Router::new()
        .route("/api/snapshot", get(snapshot::<S>))
        .route("/api/events", get(events::<S>))
        .fallback(not_found)
        .with_state(state)
}

/// Bind and serve the standard local-only dashboard endpoint.
pub(crate) async fn serve<S: SnapshotSource>(source: S, host: &str, port: u16) -> Result<()> {
    let listener = TcpListener::bind((host, port)).await.map_err(AppError::from)?;
    println!("Serving dashboard API at http://{host}:{port}");
    axum::serve(listener, router(SnapshotService::new(source)))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(AppError::from)
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn snapshot<S: SnapshotSource>(
    State(service): State<Arc<SnapshotService<S>>>,
) -> std::result::Result<Json<DashboardSnapshotV1>, ApiError> {
    Ok(Json(service.snapshot()?))
}

async fn events<S: SnapshotSource>(State(service): State<Arc<SnapshotService<S>>>) -> impl IntoResponse {
    let snapshots = stream! {
        let mut last_generation = None;
        let mut last_sent = Instant::now() - Duration::from_secs(HEARTBEAT_SECONDS);
        let mut ticker = interval(Duration::from_secs(POLL_SECONDS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let generation = match service.generation() {
                Ok(generation) => generation,
                Err(_) => continue,
            };
            if should_send(last_generation, generation, last_sent.elapsed()) &&
                let Ok(payload) = service.snapshot()
            {
                match sse_snapshot_event(&payload) {
                    Ok(event) => yield Ok::<Event, Infallible>(event),
                    Err(_) => continue,
                }
                last_generation = Some(generation);
                last_sent = Instant::now();
            }
        }
    };
    Sse::new(snapshots)
}

fn should_send(last_generation: Option<u64>, generation: u64, since_last_send: Duration) -> bool {
    last_generation != Some(generation) || since_last_send >= Duration::from_secs(HEARTBEAT_SECONDS)
}

/// Encode the exact named event framing used by the dashboard EventSource.
#[cfg(test)]
pub(crate) fn sse_snapshot_frame(payload: &impl Serialize) -> Result<String> {
    Ok(format!("event: snapshot\ndata: {}\n\n", compact_json(payload)?))
}

fn sse_snapshot_event(payload: &impl Serialize) -> Result<Event> {
    Ok(Event::default().event("snapshot").data(compact_json(payload)?))
}

fn compact_json(payload: &impl Serialize) -> Result<String> {
    // Going through Value preserves deterministic lexical object ordering.
    Ok(serde_json::to_string(&serde_json::to_value(payload)?)?)
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response()
}

struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": self.0.message }))).into_response()
    }
}

fn rfc3339_utc(time: SystemTime) -> String {
    let seconds = time.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let clock = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_date(days);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}+00:00", clock / 3_600, (clock % 3_600) / 60, clock % 60)
}

// Howard Hinnant's civil-date algorithm, adapted to signed Unix-day offsets.
fn civil_date(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use super::*;
    use crate::domain::{Identity, OutsideScopeV1, ProviderReport, SnapshotScopeKindV1, SnapshotScopeV1};

    struct Source {
        generation: AtomicU64,
        generation_reads: AtomicUsize,
        snapshots: AtomicUsize,
    }

    impl Source {
        fn new(generation: u64) -> Self {
            Self {
                generation: AtomicU64::new(generation),
                generation_reads: AtomicUsize::new(0),
                snapshots: AtomicUsize::new(0),
            }
        }
    }

    impl SnapshotSource for Source {
        fn snapshot(&self) -> Result<SnapshotV1> {
            self.snapshots.fetch_add(1, Ordering::SeqCst);
            Ok(SnapshotV1 {
                schema_version: 1,
                complete: true,
                scope: SnapshotScopeV1 { kind: SnapshotScopeKindV1::Machine, repo_root: None },
                self_identity: Some(Identity { client: Client::Codex, session_id: "self".into() }),
                providers: vec![ProviderReport {
                    client: Client::Codex,
                    ok: true,
                    source: "test".into(),
                    enabled: true,
                    dropped: 0,
                    error: None,
                }],
                sessions: vec![],
                claims: vec![],
                notes: vec![],
                delegates: vec![],
                outside_scope: OutsideScopeV1 { sessions: 0, directories: 0 },
            })
        }
        fn messages(&self) -> Result<Vec<SnapshotMessageV1>> {
            Ok(vec![])
        }
        fn generation(&self) -> Result<u64> {
            self.generation_reads.fetch_add(1, Ordering::SeqCst);
            Ok(self.generation.load(Ordering::SeqCst))
        }
    }

    struct BrokenSource;

    impl SnapshotSource for BrokenSource {
        fn snapshot(&self) -> Result<SnapshotV1> {
            Err(AppError::operational("snapshot unavailable"))
        }

        fn messages(&self) -> Result<Vec<SnapshotMessageV1>> {
            Ok(vec![])
        }

        fn generation(&self) -> Result<u64> {
            Ok(1)
        }
    }

    #[test]
    fn exact_sse_frame_is_named_and_compact() {
        assert_eq!(
            sse_snapshot_frame(&json!({ "generation": 7 })).unwrap(),
            "event: snapshot\ndata: {\"generation\":7}\n\n"
        );
    }

    #[test]
    fn snapshots_share_the_cache_but_generation_does_not() {
        let source = Source::new(3);
        let service = SnapshotService::with_cache_ttl(source, Duration::from_secs(2));
        let first = service.snapshot().unwrap();
        let second = service.snapshot().unwrap();
        assert_eq!(first, second);
        assert_eq!(service.source.snapshots.load(Ordering::SeqCst), 1);
        assert_eq!(service.generation().unwrap(), 3);
        assert_eq!(service.source.generation_reads.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn generation_changes_and_heartbeats_are_sent() {
        assert!(should_send(None, 4, Duration::ZERO));
        assert!(!should_send(Some(4), 4, Duration::from_secs(19)));
        assert!(should_send(Some(4), 5, Duration::ZERO));
        assert!(should_send(Some(4), 4, Duration::from_secs(20)));
    }

    #[test]
    fn timestamps_are_dashboard_parseable_rfc3339() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH + Duration::from_secs(1_722_729_600)), "2024-08-04T00:00:00+00:00");
    }

    #[tokio::test]
    async fn snapshot_and_404_are_json() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind((DEFAULT_HOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(SnapshotService::new(Source::new(1)))).await.unwrap();
        });

        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.write_all(b"GET /api/snapshot HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("\"schema_version\":1"));

        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.write_all(b"GET /missing HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 404"));
        assert!(response.contains("{\"error\":\"not found\"}"));
        server.abort();

        let listener = TcpListener::bind((DEFAULT_HOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(SnapshotService::new(BrokenSource))).await.unwrap();
        });
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.write_all(b"GET /api/snapshot HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 500"));
        assert!(response.contains("{\"error\":\"snapshot unavailable\"}"));
        server.abort();
    }
}
