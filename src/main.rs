use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::{app::App, logger::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    init_logger();

    info!("app starting");

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

    Ok(())
}

mod text {
    use ropey::Rope;
    use ropey::{iter::Chunks, RopeSlice};
    use std::fs::File;
    use std::io::{BufReader, BufWriter};

    pub fn get_file(file: &str) -> Result<Rope, std::io::Error> {
        Rope::from_reader(BufReader::new(File::open(file)?))
    }
}

mod app;

mod renderer;

mod experiments;

mod modes;

mod logger;
