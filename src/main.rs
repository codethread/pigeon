use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::App,
    renderer::{Line, Span, UiCtx, Widget},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let mut app = App::default();

    loop {
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::CONTROL,
                }) => break,
                Event::Key(KeyEvent { code, .. }) => app.handle(code)?,
                _ => (),
            }
        }

        app.idle()?;
    }

    // let line = Line {
    //     spans: vec![
    //         Span::new("hello there ".to_string()),
    //         Span::new("chunk".to_string()).style(crossterm::style::Attribute::Bold),
    //     ],
    // };

    // let mut stdout = std::io::stdout();
    // let mut ui_ctx = UiCtx {
    //     stdout,
    //     row_start: 7,
    //     row_end: 8,
    //     col_start: 5,
    //     col_end: 20,
    // };

    // line.render(&mut ui_ctx)?;

    Ok(())
}

mod app;

mod renderer;

mod experiments;

mod modes;
