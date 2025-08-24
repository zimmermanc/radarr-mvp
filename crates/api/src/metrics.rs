use anyhow::Result;
use axum::{
    body::Body,
    extract::MatchedPath,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, Encoder,
    GaugeVec, HistogramVec, TextEncoder,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Metrics collection using Prometheus for MVP
#[derive(Clone)]
pub struct MetricsCollector {
    // Prometheus metrics
    prom_http_requests: CounterVec,
    prom_http_duration: HistogramVec,
    prom_db_queries: CounterVec,
    prom_business_events: CounterVec,
    prom_system_gauges: GaugeVec,

    // Radarr-specific business metrics
    search_total: CounterVec,
    grab_total: CounterVec,
    import_success_total: CounterVec,
    import_failure_total: CounterVec,
    queue_length: GaugeVec,
    search_duration_seconds: HistogramVec,
}

impl MetricsCollector {
    pub fn new() -> Result<Self> {
        // Initialize Prometheus metrics
        let prom_http_requests = register_counter_vec!(
            "radarr_http_requests_total",
            "Total number of HTTP requests",
            &["method", "route", "status"]
        )?;

        let prom_http_duration = register_histogram_vec!(
            "radarr_http_request_duration_seconds",
            "HTTP request duration in seconds",
            &["method", "route", "status"],
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )?;

        let prom_db_queries = register_counter_vec!(
            "radarr_db_queries_total",
            "Total number of database queries",
            &["operation", "table", "status"]
        )?;

        let prom_business_events = register_counter_vec!(
            "radarr_business_events_total",
            "Total number of business events",
            &["event_type", "status"]
        )?;

        let prom_system_gauges =
            register_gauge_vec!("radarr_system_metrics", "System metrics", &["metric_type"])?;

        // Radarr-specific business metrics
        let search_total = register_counter_vec!(
            "radarr_search_total",
            "Total number of searches performed",
            &["indexer", "status"]
        )?;

        let grab_total = register_counter_vec!(
            "radarr_grab_total",
            "Total number of releases grabbed",
            &["indexer", "quality", "status"]
        )?;

        let import_success_total = register_counter_vec!(
            "radarr_import_success_total",
            "Total number of successful imports",
            &["quality", "source"]
        )?;

        let import_failure_total = register_counter_vec!(
            "radarr_import_failure_total",
            "Total number of failed imports",
            &["reason", "source"]
        )?;

        let queue_length = register_gauge_vec!(
            "radarr_queue_length",
            "Current length of download queue",
            &["status"]
        )?;

        let search_duration_seconds = register_histogram_vec!(
            "radarr_search_duration_seconds",
            "Duration of search operations in seconds",
            &["indexer"],
            vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
        )?;

        Ok(Self {
            prom_http_requests,
            prom_http_duration,
            prom_db_queries,
            prom_business_events,
            prom_system_gauges,
            search_total,
            grab_total,
            import_success_total,
            import_failure_total,
            queue_length,
            search_duration_seconds,
        })
    }

    /// Record HTTP request metrics
    pub fn record_http_request(
        &self,
        method: &str,
        route: &str,
        status: u16,
        duration: Duration,
        _response_size: u64,
    ) {
        let status_str = status.to_string();
        let duration_secs = duration.as_secs_f64();

        // Prometheus metrics
        self.prom_http_requests
            .with_label_values(&[method, route, &status_str])
            .inc();

        self.prom_http_duration
            .with_label_values(&[method, route, &status_str])
            .observe(duration_secs);
    }

    /// Record database query metrics
    pub fn record_db_query(
        &self,
        operation: &str,
        table: &str,
        _duration: Duration,
        success: bool,
    ) {
        let status = if success { "success" } else { "error" };

        // Prometheus metrics
        self.prom_db_queries
            .with_label_values(&[operation, table, status])
            .inc();
    }

    /// Record business event metrics
    pub fn record_business_event(&self, event_type: &str, success: bool) {
        let status = if success { "success" } else { "error" };

        // Prometheus business events
        self.prom_business_events
            .with_label_values(&[event_type, status])
            .inc();
    }

    /// Record error
    pub fn record_error(&self, error_type: &str, _context: &str) {
        self.prom_business_events
            .with_label_values(&["error", error_type])
            .inc();
    }

    /// Update database connection metrics
    pub fn update_db_connections(&self, active: i64, pool_size: u64) {
        self.prom_system_gauges
            .with_label_values(&["db_connections_active"])
            .set(active as f64);
        self.prom_system_gauges
            .with_label_values(&["db_pool_size"])
            .set(pool_size as f64);
    }

    /// Update system metrics
    pub fn update_system_metrics(&self, memory_bytes: u64, cpu_percent: f64, active_tasks: i64) {
        self.prom_system_gauges
            .with_label_values(&["memory_usage_bytes"])
            .set(memory_bytes as f64);
        self.prom_system_gauges
            .with_label_values(&["cpu_usage_percent"])
            .set(cpu_percent);
        self.prom_system_gauges
            .with_label_values(&["active_tasks"])
            .set(active_tasks as f64);
    }

    // Radarr-specific metric recording methods

    /// Record a search operation
    pub fn record_search(&self, indexer: &str, duration: Duration, success: bool) {
        let status = if success { "success" } else { "error" };

        self.search_total
            .with_label_values(&[indexer, status])
            .inc();

        self.search_duration_seconds
            .with_label_values(&[indexer])
            .observe(duration.as_secs_f64());
    }

    /// Record a release grab
    pub fn record_grab(&self, indexer: &str, quality: &str, success: bool) {
        let status = if success { "success" } else { "error" };

        self.grab_total
            .with_label_values(&[indexer, quality, status])
            .inc();
    }

    /// Record a successful import
    pub fn record_import_success(&self, quality: &str, source: &str) {
        self.import_success_total
            .with_label_values(&[quality, source])
            .inc();
    }

    /// Record a failed import
    pub fn record_import_failure(&self, reason: &str, source: &str) {
        self.import_failure_total
            .with_label_values(&[reason, source])
            .inc();
    }

    /// Update queue length
    pub fn update_queue_length(&self, queued: i64, downloading: i64, paused: i64) {
        self.queue_length
            .with_label_values(&["queued"])
            .set(queued as f64);
        self.queue_length
            .with_label_values(&["downloading"])
            .set(downloading as f64);
        self.queue_length
            .with_label_values(&["paused"])
            .set(paused as f64);
        self.queue_length
            .with_label_values(&["total"])
            .set((queued + downloading + paused) as f64);
    }

    /// Export Prometheus metrics
    pub fn export_prometheus(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        Ok(encoder.encode_to_string(&metric_families)?)
    }
}

/// Middleware for automatic HTTP metrics collection
pub async fn metrics_middleware(req: Request<Body>, next: Next) -> Result<Response, Response> {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|matched_path| matched_path.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    // Add trace information to request
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        otel.kind = "server",
        http.method = %method,
        http.route = %path,
    );

    let response = next.run(req).await;
    let duration = start.elapsed();
    let status = response.status().as_u16();

    // Record metrics if collector is available
    if let Some(metrics) = response.extensions().get::<Arc<MetricsCollector>>() {
        let response_size = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        metrics.record_http_request(&method, &path, status, duration, response_size);
    }

    // Add metrics to span
    span.record("http.status_code", status);
    span.record(
        "http.response_size",
        response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("0"),
    );
    span.record("duration_ms", duration.as_millis() as u64);

    Ok(response)
}

/// Health check endpoint that includes metrics
pub async fn health_with_metrics(
    metrics: axum::extract::Extension<Arc<MetricsCollector>>,
) -> impl IntoResponse {
    use axum::Json;
    use serde_json::json;

    // Update system metrics
    let memory_info = sys_info::mem_info().unwrap_or_default();
    let memory_bytes = memory_info.total * 1024; // Convert KB to bytes

    let cpu_percent = sys_info::loadavg()
        .map(|load| load.one * 100.0)
        .unwrap_or(0.0);

    metrics.update_system_metrics(memory_bytes, cpu_percent, 0);

    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "metrics": {
            "memory_bytes": memory_bytes,
            "cpu_percent": cpu_percent,
        }
    }))
}

/// Prometheus metrics endpoint
pub async fn prometheus_metrics(
    metrics: axum::extract::Extension<Arc<MetricsCollector>>,
) -> impl IntoResponse {
    match metrics.export_prometheus() {
        Ok(metrics_text) => (
            axum::http::StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4")],
            metrics_text,
        ),
        Err(e) => {
            tracing::error!("Failed to export Prometheus metrics: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                format!("Error exporting metrics: {}", e),
            )
        }
    }
}

/// Add sys-info dependency for system metrics
/// This would need to be added to Cargo.toml
pub mod sys_info {
    pub struct MemInfo {
        pub total: u64,
        pub free: u64,
        pub avail: u64,
    }

    impl Default for MemInfo {
        fn default() -> Self {
            Self {
                total: 0,
                free: 0,
                avail: 0,
            }
        }
    }

    pub struct LoadAvg {
        pub one: f64,
        pub five: f64,
        pub fifteen: f64,
    }

    pub fn mem_info() -> anyhow::Result<MemInfo> {
        // Placeholder - would use actual sys-info crate
        Ok(MemInfo::default())
    }

    pub fn loadavg() -> anyhow::Result<LoadAvg> {
        // Placeholder - would use actual sys-info crate
        Ok(LoadAvg {
            one: 0.0,
            five: 0.0,
            fifteen: 0.0,
        })
    }
}
