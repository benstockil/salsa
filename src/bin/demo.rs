use std::error::Error;

use choreo::{
    coordinates::GroupCoordinates,
    iterator::DanceIterator,
    parser::{Lexer, Parser},
};
use nannou::prelude::*;

fn main() -> Result<(), Box<dyn Error>> {
    nannou::app(model).update(update).run();

    Ok(())
}

struct AppModel {
    window: window::Id,
    start: f32,
    iterator: DanceIterator,
    positions: GroupCoordinates,
}

fn model(app: &App) -> AppModel {
    let window = app.new_window().view(view).build().unwrap();
    let lexer = Lexer::from_file("./first_dance.csf").unwrap();
    let parser = Parser::new(lexer);

    let script = parser.parse().unwrap();
    let mut iterator = DanceIterator::from(script);

    dbg!(&iterator);

    AppModel {
        start: app.time,
        window,
        positions: iterator.advance_to(0.0),
        iterator,
    }
}

fn update(app: &App, model: &mut AppModel, _update: Update) {
    model.positions = model.iterator.advance_to((app.time - model.start) as f64);
}

fn view(app: &App, model: &AppModel, frame: Frame) {
    let dance = model.iterator.dance();
    let stage = dance.meta.stage_size;
    // let iterator = DanceIterator::from(dance.to_owned());

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
