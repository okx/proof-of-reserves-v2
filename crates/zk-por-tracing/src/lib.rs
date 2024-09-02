pub mod opentelemetry;

use crate::opentelemetry::exporter::stdout;
use tracing::Level;
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_flame::FlameLayer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{fmt, prelude::*};

#[derive(Debug)]
pub struct TraceConfig {
    pub prefix: String,
    pub dir: String,
    pub level: Level,
    pub console: bool,
    pub flame: bool,
}

pub fn init_tracing(config: TraceConfig) -> Vec<WorkerGuard> {
    let prefix = config.prefix;
    let dir = config.dir;
    let level = config.level;
    let console = config.console;
    let flame = config.flame;

    let mut guards = vec![];
    let (fmt_writer, fmt_guard) = tracing_appender::non_blocking(rolling::daily(&dir, &prefix));
    guards.push(fmt_guard);
    let (telemetry_writer, telemetry_guard) =
        tracing_appender::non_blocking(rolling::daily(&dir, prefix.clone() + ".telemetry"));
    guards.push(telemetry_guard);
    let tracer = stdout::new_pipeline().with_writer(telemetry_writer).install_simple();
    let layered = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(fmt_writer)
        .with_ansi(false)
        .finish()
        .with(OpenTelemetryLayer::new(tracer));
    if flame {
        let (folded_writer, folded_guard) =
            tracing_appender::non_blocking(rolling::daily(&dir, prefix.clone() + ".folded"));
        guards.push(folded_guard);
        if console {
            layered.with(fmt::Layer::default()).with(FlameLayer::new(folded_writer)).init();
        } else {
            layered.with(FlameLayer::new(folded_writer)).init()
        }
    } else if console {
        layered.with(fmt::Layer::default()).init();
    } else {
        layered.init()
    }

    guards
}
