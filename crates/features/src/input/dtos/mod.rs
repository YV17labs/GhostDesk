//! What a client sends for the seven input tools, and the verdict all seven
//! answer with.

mod button_dto;
mod click_dto;
mod drag_dto;
mod feedback_dto;
mod move_dto;
mod press_dto;
mod scroll_direction_dto;
mod scroll_dto;
mod type_dto;

pub use button_dto::ButtonDto;
pub use click_dto::ClickDto;
pub use drag_dto::DragDto;
pub use feedback_dto::FeedbackDto;
pub use move_dto::MoveDto;
pub use press_dto::PressDto;
pub use scroll_direction_dto::ScrollDirectionDto;
pub use scroll_dto::ScrollDto;
pub use type_dto::TypeDto;
