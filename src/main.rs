//! Runs the loom engine's default application.

#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

use loom::prelude::*;

/// Opens the engine window and runs until it is closed.
fn main() {
    App::new().run();
}
