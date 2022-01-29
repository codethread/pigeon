use crossterm::cursor::{Hide, MoveTo, MoveToNextLine, RestorePosition, SavePosition, Show};
use crossterm::style::{
    Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, Clear, ClearType, DisableLineWrap,
};
use crossterm::{ExecutableCommand, QueueableCommand};
use log::info;
use std::fmt::Display;
use std::io::{self, Error, Stdout, Write};

use crate::modes::Cursor;

pub type RendResult = Result<(), std::io::Error>;

mod widgets;

use self::widgets::*;

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

        let (columns, rows) = terminal::size().expect("could not get terminal size");

        Self { stdout }
    }

    pub fn render<T: Widget>(&mut self, screen: &mut T) -> RendResult {
        let (columns, rows) = terminal::size().expect("could not get terminal size");

        let mut ctx = UiCtx {
            stdout: &mut self.stdout,
            row_start: 0,
            row_end: 1,
            col_start: 0,
            col_end: columns,
        };

        screen.render(&mut ctx)?;

        ctx.stdout.flush()?;

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

        info!("App dropped\n\n");

        disable_raw_mode().unwrap();
    }
}

#[derive(Debug)]
pub struct UI {
    widgets: List,
    windows: Vec<Window>,
    active_window: usize,
}

impl Default for UI {
    fn default() -> Self {
        let mut main_window = Window::default();
        let (cols, rows) = terminal::size().expect("could not get terminal size");

        let cols = cols.try_into().unwrap();
        let rows = rows.try_into().unwrap();

        main_window.resize(rows, cols);
        Self {
            widgets: Default::default(),
            windows: vec![main_window],
            active_window: 0,
        }
    }
}

impl UI {
    pub fn get_active_window(&mut self) -> &mut Window {
        self.windows
            .get_mut(self.active_window)
            .expect("could not get active window")
    }

    // temporary
    pub fn screen(&mut self) -> &mut Window {
        self.get_active_window()
    }
}

#[derive(Default, Debug)]
pub struct Window {
    buffer: usize,
    contents: List,
    did_update: bool,
    rows: usize,
    cols: usize,
}

use crate::app::buffer::Buffer;

impl Window {
    pub fn set_buffer(&mut self, buff_id: usize, buffer: &Buffer) {
        self.buffer = buff_id;

        // start simple and set row as start of view
        let Cursor { row, .. } = buffer.get_cursor();

        let lines = buffer.get_lines_range(*row, row + self.rows);
        let mut list = List::from(lines);
        list.expand(self.rows);
        self.contents = list;
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.rows = rows;
        self.cols = cols;
    }
}

impl Widget for Window {
    type Components = List;

    fn render(&mut self, ui_ctx: &mut UiCtx) -> RendResult {
        self.contents.render(ui_ctx)
    }

    fn did_update(&self) -> bool {
        self.did_update
    }

    fn update(&mut self, cb: impl FnMut(&mut Self::Components)) {
        todo!()
    }
}
