use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
pub use tracing_appender::non_blocking::WorkerGuard as LogGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::time::LocalTime;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
// 重导出 tracing 的宏，这样其他模块只需 use niupanel_common::logger::*;
pub use tracing::{Level, debug, error, info, span, trace, warn};

pub struct LogConfig {
    pub level: String,
    pub filters: String,
    pub log_dir: Option<PathBuf>,
    pub file_prefix: String,
}

pub fn init(config: LogConfig) -> Option<WorkerGuard> {
    let mut filter_str = config.level.clone();
    if !config.filters.is_empty() {
        filter_str = format!("{},{}", filter_str, config.filters);
    }

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::try_new(&filter_str).unwrap_or_else(|_| EnvFilter::new("warn"))
    });

    // Keep noisy dependencies at production-safe levels while respecting the
    // configured global level (for example, `warn` in production and `info`
    // in development).
    let env_filter = env_filter
        .add_directive("sqlx=warn".parse().unwrap())
        .add_directive("sea_orm=warn".parse().unwrap())
        .add_directive("tower_http=warn".parse().unwrap());

    // 自定义时间格式: [2025-12-22 14:30:01]
    let timer = LocalTime::new(time::macros::format_description!(
        "[year]/[month]/[day] [hour]:[minute]:[second]"
    ));

    // 控制台图层
    let console_layer = fmt::layer()
        .with_timer(timer.clone())
        .with_target(true)
        .with_thread_ids(false)
        .with_line_number(false) // 保持简洁
        .with_file(false);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    // 如果配置了目录，则添加文件图层
    if let Some(log_dir) = config.log_dir {
        if !log_dir.exists() {
            std::fs::create_dir_all(&log_dir).ok();
        }

        // 使用 Builder 模式以产生 niupanel.YYYY-MM-DD.log 格式
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(&config.file_prefix)
            .filename_suffix("log")
            .build(log_dir)
            .expect("Failed to create rolling file appender");

        // 使用非阻塞写入提高性能
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_timer(timer)
            .with_ansi(false) // 文件日志不需要颜色代码
            .with_target(true)
            .with_line_number(false) // 文件日志也保持简洁
            .with_file(false)
            .with_writer(non_blocking);

        registry.with(file_layer).init();
        info!(
            "Logger initialized with file output, filter: {}",
            filter_str
        );
        Some(guard)
    } else {
        registry.init();
        info!(
            "Logger initialized with console output only, filter: {}",
            filter_str
        );
        None
    }
}
