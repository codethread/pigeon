use crossterm::cursor::{Hide, MoveTo, MoveToNextLine, RestorePosition, SavePosition, Show};
use crossterm::style::{
    Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, DisableLineWrap};
use crossterm::{ExecutableCommand, QueueableCommand};
use std::fmt::Display;
use std::io::{self, Error, Stdout, Write};

use crate::modes::Cursor;

pub type RendResult = Result<(), std::io::Error>;

pub struct UiCtx {
    pub stdout: Stdout,
    pub row_start: u16,
    pub row_end: u16,
    pub col_start: u16,
    pub col_end: u16,
}

pub trait Widget {
    fn render(&self, ui_ctx: &mut UiCtx) -> RendResult;
}

pub struct Span {
    msg: String,
    style: Option<Attribute>,
    fg: Option<Color>,
    bg: Option<Color>,
}

impl Span {
    pub fn new(msg: String) -> Self {
        Self {
            msg,
            style: None,
            fg: None,
            bg: None,
        }
    }

    pub fn style(mut self, atrr: Attribute) -> Self {
        self.style = Some(atrr);
        self
    }
}

impl Widget for Line {
    fn render(&self, ui_ctx: &mut UiCtx) -> RendResult {
        let mut r = &ui_ctx.stdout;

        // TODO simple clear for now
        r.queue(MoveTo(ui_ctx.row_start, ui_ctx.col_start))?;
        r.queue(Print(format!(
            "{:>width$}",
            " ",
            width = (ui_ctx.col_end - ui_ctx.col_start) as usize
        )))?;

        self.spans.iter().for_each(|Span { fg, bg, msg, style }| {
            fg.map(|c| r.queue(SetForegroundColor(c)).unwrap());
            bg.map(|c| r.queue(SetBackgroundColor(c)).unwrap());
            style.map(|c| r.queue(SetAttribute(c)).unwrap());

            r.queue(Print(msg)).unwrap();

            fg.map(|_| r.queue(SetForegroundColor(Color::Black)).unwrap());
            bg.map(|_| r.queue(SetBackgroundColor(Color::Reset)).unwrap());
            style.map(|_| r.queue(SetAttribute(Attribute::Reset)).unwrap());
        });

        Ok(())
    }
}

pub struct Line {
    pub spans: Vec<Span>,
}

#[derive(Debug)]
pub struct Renderer {
    stdout: Stdout,
}

impl Renderer {
    pub fn new() -> Self {
        let mut stdout = io::stdout();

        enable_raw_mode().unwrap();

        #[inline]
        fn clear(stdout: &mut Stdout) -> RendResult {
            stdout
                // .queue(EnterAlternateScreen)?
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

    pub fn paint<T: Display>(&mut self, msg: &T) -> RendResult {
        self.msg(msg)?;

        Ok(())
    }

    pub fn prompt(&mut self) -> RendResult {
        self.msg(&"choose (a): cat, (b): dog, (c): fish")?;

        Ok(())
    }

    pub fn draw(&mut self, ui: String) -> RendResult {
        self.stdout.queue(MoveTo(0, 0))?;

        for line in ui.lines() {
            self.stdout
                .queue(Clear(ClearType::CurrentLine))?
                .queue(Print(line))?
                .queue(MoveToNextLine(1))?;
        }

        self.stdout.flush()?;

        Ok(())
    }

    fn msg<T: Display>(&mut self, msg: &T) -> RendResult {
        self.stdout
            .queue(Print("---------------"))?
            .queue(MoveToNextLine(1))?
            .queue(Print(msg))?
            .queue(MoveToNextLine(1))?
            .flush()?;

        Ok(())
    }

    pub fn cursor(&mut self, cursor: &Cursor) -> RendResult {
        let (r, c) = cursor.as_u16().unwrap();
        self.stdout
            .queue(SavePosition)?
            .queue(MoveTo(c, r + 1))?
            .queue(Print("_"))?
            .queue(RestorePosition)?
            .flush()
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
            // .execute(LeaveAlternateScreen)
            // .unwrap()
            .execute(Show)
            .unwrap();

        disable_raw_mode().unwrap();
    }
}
