//! Loom is a minimal game engine.
//!
//! The current surface is small: [`App`](prelude::App) opens a window and
//! runs a game loop with an update rate decoupled from its render rate.
//! Audio, game state, input handling, and networking are not implemented
//! yet.
//!
//! ```no_run
//! use loom::prelude::*;
//!
//! App::new().run();
//! ```

#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

pub mod app;
pub mod ecs;
pub mod prelude;
