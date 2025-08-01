pub mod config;
pub mod durability;
pub mod error;
pub mod utils;

wit_bindgen::generate!({
    path: "../wit",
    world: "video-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue],
    pub_export_macro: true,
});

pub use __export_video_library_impl as export_video;
use std::cell::RefCell;
use std::str::FromStr;

pub struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    /// Initializes WASI logging based on the `GOLEM_VIDEO_LOG` environment variable.
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter =
                log::LevelFilter::from_str(&std::env::var("GOLEM_VIDEO_LOG").unwrap_or_default())
                    .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    /// This holds the state of our application.
    static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
}
