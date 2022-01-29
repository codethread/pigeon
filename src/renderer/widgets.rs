use crossterm::cursor::MoveTo;
use crossterm::style::{
    Attribute, Color, Colors, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use log::info;
use std::io::{self, Error, Stdout, Write};
use std::str::FromStr;

use super::RendResult;
use crossterm::{ExecutableCommand, QueueableCommand};

#[derive(Debug)]
pub struct UiCtx<'a> {
    pub stdout: &'a mut Stdout,
    pub row_start: u16,
    pub row_end: u16,
    pub col_start: u16,
    pub col_end: u16,
}

pub trait Widget {
    type Components;

    fn render(&mut self, ui_ctx: &mut UiCtx) -> RendResult;

    fn did_update(&self) -> bool;

    fn update(&mut self, cb: impl FnMut(&mut Self::Components));
}

#[derive(Debug)]
pub struct Span {
    msg: String,
    style: Option<Attribute>,
    fg: Option<Color>,
    bg: Option<Color>,
}

impl Default for Span {
    fn default() -> Self {
        Self::new("".to_string())
    }
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

    pub fn color(mut self, col: Color) -> Self {
        self.fg = Some(col);
        self
    }
}

#[derive(Debug)]
pub struct WigError {}

impl FromStr for Span {
    type Err = WigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

impl Widget for Line {
    type Components = Vec<Span>;

    fn did_update(&self) -> bool {
        self.did_update
    }

    fn render(&mut self, ui_ctx: &mut UiCtx) -> RendResult {
        if !self.did_update() {
            return Ok(());
        }

        let r = &mut ui_ctx.stdout;

        // TODO simple clear for now
        // r.queue(MoveTo(ui_ctx.row_start, ui_ctx.col_start))?;
        r.queue(MoveTo(ui_ctx.col_start, ui_ctx.row_start))?;
        // r.queue(Print(format!(
        //     "{:>width$}",
        //     " ",
        //     width = (ui_ctx.col_end - ui_ctx.col_start) as usize
        // )))?;

        self.spans.iter().for_each(|Span { fg, bg, msg, style }| {
            fg.map(|c| r.queue(SetForegroundColor(c)).unwrap());
            bg.map(|c| r.queue(SetBackgroundColor(c)).unwrap());
            style.map(|c| r.queue(SetAttribute(c)).unwrap());

            r.queue(Print(msg)).unwrap();

            fg.map(|_| r.queue(SetForegroundColor(Color::Black)).unwrap());
            bg.map(|_| r.queue(SetBackgroundColor(Color::Reset)).unwrap());
            style.map(|_| r.queue(SetAttribute(Attribute::Reset)).unwrap());
        });

        self.did_update = false;

        Ok(())
    }

    fn update(&mut self, mut cb: impl FnMut(&mut Self::Components)) {
        cb(&mut self.spans);
        self.did_update = true;
    }
}

#[derive(Default, Debug)]
pub struct Line {
    pub spans: Vec<Span>,
    did_update: bool,
}

impl Line {
    pub fn new(spans: Vec<Span>) -> Self {
        Self {
            spans,
            did_update: true,
        }
    }

    fn empty() -> Line {
        let tilde = Span::new("~".to_string()).color(Color::Cyan);

        Self {
            did_update: true,
            spans: vec![tilde],
        }
    }
}

impl FromStr for Line {
    type Err = WigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let span = Span::from_str(s).unwrap();
        Ok(Self::new(vec![span]))
    }
}

#[derive(Default, Debug)]
pub struct List {
    pub lines: Vec<Line>,
    did_update: bool,
}

impl List {
    pub fn new(lines: Vec<Line>) -> Self {
        Self {
            lines,
            did_update: true,
        }
    }

    // expand current list to specified number of rows
    pub fn expand(&mut self, size: usize) {
        let current_size = self.lines.len();
        info!("expanding from {} to {}", current_size, size);

        if current_size < size {
            for _ in 0..(size - current_size) {
                self.lines.push(Line::empty())
            }
        }
    }
}

impl Widget for List {
    type Components = Vec<Line>;

    fn render(&mut self, ui_ctx: &mut UiCtx) -> RendResult {
        for line in self.lines.iter_mut() {
            line.render(ui_ctx).unwrap();
            ui_ctx.row_start += 1;
            ui_ctx.row_end += 1;
        }

        Ok(())
    }

    fn did_update(&self) -> bool {
        self.did_update
    }

    fn update(&mut self, mut cb: impl FnMut(&mut Self::Components)) {
        cb(&mut self.lines);
        self.did_update = true;
    }
}

use crate::app::buffer::Lines;

impl From<Lines<'_>> for List {
    fn from(lines: Lines) -> Self {
        let Lines { lines, empty } = lines;

        let mut list: Vec<Line> = Vec::new();

        for line in lines.lines() {
            let l = Line::from_str(line.as_str().unwrap()).unwrap();
            list.push(l);
        }

        Self::new(list)
    }
}
