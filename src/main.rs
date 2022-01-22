use crossterm::cursor::{Hide, MoveTo, MoveToNextLine, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, Clear, ClearType, DisableLineWrap,
    EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{ExecutableCommand, QueueableCommand};
use rust_fsm::*;
use std::io::{self, Error, Stdout, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let mut app = App::default();

    loop {
        match event::read()? {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            }) => break,
            Event::Key(KeyEvent {
                code: KeyCode::Char(k),
                ..
            }) => app.handle(k)?,
            _ => (),
        }
    }

    Ok(())
}

type RendResult = Result<(), std::io::Error>;

#[derive(Default, Debug)]
struct App {
    renderer: Renderer,
}

impl App {
    pub fn handle(&mut self, k: char) -> RendResult {
        self.renderer.paint(k)?;
        Ok(())
    }
}

#[derive(Debug)]
struct Renderer {
    stdout: Stdout,
}

impl Renderer {
    pub fn new() -> Self {
        let mut stdout = io::stdout();

        enable_raw_mode().unwrap();

        #[inline]
        fn clear(stdout: &mut Stdout) -> RendResult {
            stdout
                .queue(EnterAlternateScreen)?
                .queue(DisableLineWrap)?
                .queue(Hide)?
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(0, 0))?
                .flush()?;

            Ok(())
        }

        clear(&mut stdout).unwrap();

        Self { stdout }
    }

    pub fn paint(&mut self, c: char) -> RendResult {
        self.stdout
            .queue(Print("---------------"))?
            .queue(MoveToNextLine(1))?
            .queue(Print(c))?
            .queue(MoveToNextLine(1))?
            .flush()?;

        Ok(())
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.stdout
            .execute(LeaveAlternateScreen)
            .unwrap()
            .execute(Show)
            .unwrap();

        disable_raw_mode().unwrap();
    }
}
