mod model;
mod screen_areas;
mod update;

pub use model::{LogFileIndexingStatus, Model, StatusMessage};
pub use screen_areas::ScreenAreas;
pub use update::{Effect, handle_job_notification, handle_key, resize_log_view};
