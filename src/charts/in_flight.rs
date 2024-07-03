use super::*;

/// writes a chart of in-flight requests to stdout.
//
//  NB: it's a little icky that this writes to stdout. the `textplots` interface doesn't give us a
//  particularly easy way to work around this.
pub fn render_in_flight(summary: &Summary) {
    let (start, end) = summary.time_range();
    let in_flight = summary
        .compute_in_flight_observations()
        .into_iter()
        .map(|(instant, count)| {
            let x = instant.duration_since(start).as_millis() as f32;
            let y = count as f32;
            (x, y)
        })
        .collect::<Vec<(f32, f32)>>();

    // compute the dimensions of our in-flight chart.
    let (xmin, ymin) = (0.0, 0.0); // use (0,0) as our origin.
    let xmax = end.duration_since(start).as_millis() as f32;
    let ymax = {
        // floats are not `Ord` so we calculate the max ourselves.
        let mut max = 0.0;
        for (_, y) in &in_flight {
            #[allow(clippy::single_match)]
            match y.total_cmp(&max) {
                std::cmp::Ordering::Greater => max = *y,
                _ => {}
            }
        }
        max
    };

    // configure and render a chart.
    println!("number of in-flight workers:");
    let line = Shape::Lines(&in_flight);
    let mut chart = textplots::Chart::new_with_y_range(
        super::dimensions::WIDTH,
        super::dimensions::HEIGHT,
        xmin,
        xmax,
        ymin,
        ymax,
    );
    chart.axis();
    chart.lineplot(&line).display();
}
