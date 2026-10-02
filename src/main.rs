use std::io::IsTerminal;
use std::process::ExitCode;
use std::sync::OnceLock;

use choreo::{
    coordinates::GroupCoordinates,
    dance::DanceScript,
    iterator::DanceIterator,
    parser::{Lexer, Parser, render_diagnostic},
};
use nannou::prelude::*;

/// The validated script, handed to nannou's `model` fn (which must be a bare
/// fn pointer and so cannot capture it directly).
static SCRIPT: OnceLock<DanceScript> = OnceLock::new();

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: choreo <file.csf>");
        return ExitCode::from(2);
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("error: could not read {path}: {err}");
            return ExitCode::from(2);
        }
    };

    match Parser::new(Lexer::from_source(&source)).parse() {
        Ok(script) => {
            let _ = SCRIPT.set(script);
            nannou::app(model).update(update).run();
            ExitCode::SUCCESS
        }
        Err(errors) => {
            report_errors(&path, &source, errors);
            ExitCode::FAILURE
        }
    }
}

/// Print all recovered parse errors as rustc-style diagnostics, in source order.
fn report_errors(path: &str, source: &str, mut errors: Vec<choreo::parser::ParseError>) {
    errors.sort_by_key(|error| (error.span.line(), error.span.col()));

    let color = std::io::stdout().is_terminal();
    let (red, reset) = if color {
        ("\x1b[1;31m", "\x1b[0m")
    } else {
        ("", "")
    };

    let mut out = String::new();
    for error in &errors {
        out.push_str(&render_diagnostic(error, path, source, color));
        out.push('\n');
    }

    let count = errors.len();
    let plural = if count == 1 { "" } else { "s" };
    out.push_str(&format!(
        "{red}error{reset}: aborting due to {count} previous error{plural}\n"
    ));

    print!("{out}");
}

struct AppModel {
    start: f32,
    iterator: DanceIterator,
    positions: GroupCoordinates,
}

fn model(app: &App) -> AppModel {
    app.new_window().view(view).build().unwrap();

    let script = SCRIPT
        .get()
        .expect("script is set before the app runs")
        .clone();
    let mut iterator = DanceIterator::from(script);
    let positions = iterator.advance_to(0.0);

    AppModel {
        start: app.time,
        iterator,
        positions,
    }
}

fn update(app: &App, model: &mut AppModel, _update: Update) {
    model.positions = model.iterator.advance_to((app.time - model.start) as f64);
}

fn view(app: &App, model: &AppModel, frame: Frame) {
    let dance = model.iterator.dance();
    let stage = dance.meta.stage_size;

    let area = frame.rect().pad(50.0);
    let scale = f32::min(
        area.w() / (stage.width + 2) as f32,
        area.h() / (stage.depth + 2) as f32,
    );

    let draw = app.draw();
    draw.background().color(BLACK);

    draw.rect()
        .w((stage.width + 2) as f32 * scale)
        .h((stage.depth + 2) as f32 * scale)
        .color(rgb::<u8>(10, 10, 10));

    let xoff = stage.width as f32 * scale * 0.5;
    let yoff = stage.depth as f32 * scale * 0.5;

    for i in 0..=stage.width {
        for j in 0..=stage.depth {
            let xpos = i as f32 * scale - xoff;
            let ypos = j as f32 * scale - yoff;
            draw.ellipse()
                .radius(0.03 * scale)
                .x(xpos)
                .y(ypos)
                .color(GREY);
        }
    }

    for (id, dancer) in dance.dancers.iter() {
        let pos = model.positions.0[id];

        draw.ellipse()
            .radius(scale * 0.4)
            .x(pos.x as f32 * scale)
            .y(-pos.y as f32 * scale)
            .color(Rgba {
                color: dancer.color.into(),
                alpha: 180,
            });
    }

    draw.to_frame(app, &frame).unwrap();
}
