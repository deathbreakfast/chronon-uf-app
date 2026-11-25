//! Section components used by the job creation page.

pub mod actions;
pub mod advanced_options;
pub mod basic_info;
pub mod form_section_card;
pub mod parameters;
pub mod schedule;

pub use actions::ActionsSection;
pub use advanced_options::AdvancedOptionsSection;
pub use basic_info::BasicInfoSection;
pub use form_section_card::FormSectionCard;
pub use parameters::{ParametersSection, TypedParamFields};
pub use schedule::{RunOnceFields, ScheduleSection};
