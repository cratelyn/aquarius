use {
    super::*,
    comfy_table::{presets::UTF8_FULL, Row, Table},
    lazy_static::lazy_static,
    std::ops::Deref,
};

lazy_static! {
    static ref HEADER: Row = vec!["name", "value"].into();
}

/// render a table containing statistics about a load test.
pub fn render_table(summary: &Summary) {
    let mut table = Table::new();

    table
        .set_width(dimensions::WIDTH as u16)
        .set_header(HEADER.deref().to_owned())
        .load_preset(UTF8_FULL);

    // add a row containing the success rate. how many requests were 2XX's?
    let success_rate: Row = {
        const NAME: &str = "success rate (percentage)";
        let rate = summary.success_rate();
        [NAME.to_owned(), format!("{rate}%")].into()
    };
    table.add_row(success_rate);

    // add a row containing the median duration.
    let duration_median: Row = {
        const NAME: &str = "duration (median)";
        let duration = summary.median_duration().as_micros();
        [NAME.to_owned(), format!("{duration}µs")].into()
    };
    table.add_row(duration_median);

    // add a row containing the average number of in-flight requests.
    let in_flight_average: Row = {
        const NAME: &str = "in-flight (average)";
        let avg = summary.average_in_flight().to_string();
        [NAME.to_owned(), avg].into()
    };
    table.add_row(in_flight_average);

    // print the table
    println!("{table}");
}
