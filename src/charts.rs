//! charts and other reporting facilities.

use {
    crate::summary::Summary,
    textplots::{Plot, Shape},
};

pub use self::{in_flight::render_in_flight, progress::render_progress, table::render_table};

/// charts the number of in-flight jobs.
mod in_flight;

/// charts the progress over time.
mod progress;

/// display a table
mod table;

/// chart dimensions.
//
//  TODO: for now these are hard-coded for simplicity.
mod dimensions {
    /// the width of rendered charts.
    pub const WIDTH: u32 = 256;

    /// the height of rendered charts.
    pub const HEIGHT: u32 = 64;
}
