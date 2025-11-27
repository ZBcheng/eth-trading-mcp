use tower_http::LatencyUnit;
use tower_http::trace::{
    DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, HttpMakeClassifier, TraceLayer,
};
use tracing::Level;

pub fn http_trace_layer() -> TraceLayer<HttpMakeClassifier> {
    TraceLayer::new_for_http()
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(LatencyUnit::Millis),
        )
        .on_failure(DefaultOnFailure::new().level(Level::ERROR))
}
