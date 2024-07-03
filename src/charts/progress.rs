use super::*;

/// writes a chart of progress over time to stdout.
pub fn render_progress(summary: &Summary) {
    let (start, end) = summary.time_range();
    let finished = summary
        .compute_progress_observations()
        .into_iter()
        .map(|(instant, prog)| {
            let x = instant.duration_since(start).as_millis() as f32;
            let y = prog;
            (x, y)
        })
        .collect::<Vec<(f32, f32)>>();

    // compute the dimensions of our in-flight chart.
    let (xmin, ymin) = (0.0, 0.0); // use (0,0) as our origin.
    let xmax = end.duration_since(start).as_millis() as f32;
    let ymax = 100.0;

    // configure and render a chart.
    println!("percentage of finished workers:");
    let line = Shape::Lines(&finished);
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
