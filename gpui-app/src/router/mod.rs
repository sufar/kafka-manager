//! Router Module
//!
//! Handles view navigation and state with query params support.

mod router;

pub use router::{
    Router, ViewType, NavigationParams, NavigationState,
};