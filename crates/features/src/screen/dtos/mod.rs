//! What a client sends for `screen_shot`, and the form a capture reaches it
//! in.

mod capture_dto;
mod image_format_dto;
mod region_dto;
mod screen_shot_dto;

pub use capture_dto::CaptureDto;
pub use image_format_dto::ImageFormatDto;
pub use region_dto::RegionDto;
pub use screen_shot_dto::ScreenShotDto;
