mod error;
mod repository;
mod room_registry;
mod service;
mod voice_channel;

pub use error::VoiceError;
pub use repository::VoiceRepository;
pub use room_registry::{JoinOutcome, RoomRegistry};
pub use service::VoiceService;
pub use voice_channel::VoiceChannel;

#[cfg(test)]
mod voice_test;
