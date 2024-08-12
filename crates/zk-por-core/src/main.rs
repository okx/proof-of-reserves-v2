use tracing::{debug, Level};
use zk_por_tracing::{init_tracing, TraceConfig};

pub fn main() {
    let cfg = TraceConfig {
        prefix: "zkpor".to_string(),
        dir: "logs".to_string(),
        level: Level::DEBUG,
        console: true,
        flame: false,
    };
    let guard = init_tracing(cfg);
    debug!("tracing works");
    drop(guard)
}
