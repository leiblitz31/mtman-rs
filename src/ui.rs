//! UI layer.

use android_activity::AndroidApp;
use log::info;

/// UI state.
pub struct Ui {
    // TODO: implement
}

impl Ui {
    /// Create a new UI.
    pub fn new(_app: &AndroidApp) -> Self {
        Self {}
    }

    /// Redraw.
    pub fn redraw(&mut self, _app: &AndroidApp) {
        // TODO: implement
    }

    /// Render a frame.
    pub fn frame(&mut self, _app: &AndroidApp) {
        // TODO: implement
    }
}
