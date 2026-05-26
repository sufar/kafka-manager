//! Application State Module
//!
//! This module contains all the state management for the application.

mod app_state;
mod cluster_state;
mod favorites_state;
mod message_buffer;
mod global_state;

pub use app_state::*;
pub use favorites_state::{FavoritesState, FavoriteGroup, FavoriteItem};
pub use message_buffer::{MessageBuffer, MessageBufferConfig, BufferedMessage, MessageSizeEstimate};
pub use global_state::{GlobalState, Language};
