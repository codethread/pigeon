use crossterm::terminal;

use crate::app::AppEvent;

pub struct Ctx<'a> {
    pub modes: &'a mut Modes,
}

/// All modes known to our app
#[derive(Default, Debug)]
pub struct Modes {
    pub ui: UiMode,
    pub buff: BufferMode,
    pub modeline: ModelineMode,
}

#[derive(Debug)]
pub struct BufferMode {
    pub buffers: Vec<Buffer>,
    active: usize,
}

impl BufferMode {
    // how do I emit events?
    pub fn insert_self(&mut self, ui_mode: &mut UiMode, msg: &str, cbs: &mut Vec<AppEvent>) {
        let active_buffer = self.get_active_buffer();

        active_buffer.insert(msg);

        ui_mode.update_body(active_buffer.content.clone());

        ui_mode.update_cursor(|_| {
            Cursor::from_cursor_and_scroll_offset(&active_buffer.cursor, &active_buffer.scroll)
        });

        cbs.push(AppEvent::Move(4, 1));
    }

    pub fn insert_newline(&mut self, ui_mode: &mut UiMode) {
        let active_buffer = self.get_active_buffer();

        active_buffer.newline();

        ui_mode.update_body(active_buffer.content.clone());

        ui_mode.update_cursor(|_| {
            Cursor::from_cursor_and_scroll_offset(&active_buffer.cursor, &active_buffer.scroll)
        });
    }

    fn get_active_buffer(&mut self) -> &mut Buffer {
        self.buffers
            .get_mut(self.active)
            .expect("active buffer does not exist")
    }

    pub fn backspace(&mut self, ui_mode: &mut UiMode) {
        let active_buffer = self.get_active_buffer();

        active_buffer.backspace();

        ui_mode.update_body(active_buffer.content.clone());
        // ui_mode.update_cursor(|curs| Cursor {
        //     col: if curs.col > 1 { curs.col - 1 } else { curs.col },
        //     ..*curs
        // });
        ui_mode.update_cursor(|_| {
            Cursor::from_cursor_and_scroll_offset(&active_buffer.cursor, &active_buffer.scroll)
        });
    }
}

impl Default for BufferMode {
    fn default() -> Self {
        let mut first_buff = Buffer::default();
        first_buff.insert("type away > ");

        Self {
            buffers: vec![first_buff],
            active: 0,
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    pub content: Vec<String>,
    pub cursor: Cursor, // start simple, handle array later
    pub scroll: Pos,
}

impl Buffer {
    fn insert(&mut self, content: &str) {
        if let Some(line) = self.content.get_mut(self.cursor.row) {
            self.cursor.col += content.len();
            *line += content;
        }
    }

    fn newline(&mut self) {
        self.content.push("".into());
        self.cursor.row += 1;
        self.cursor.col = 0;
    }

    fn backspace(&mut self) {
        // maybe need to get this to report back if it went up a line
        if let Some(line) = self.content.get_mut(self.cursor.row) {
            match line.pop() {
                Some(_) => self.cursor.col -= 1,
                None => {
                    // remove last line and move cursor back up
                    if self.content.len() > 1 {
                        self.content.pop();
                        self.cursor.row -= 1;

                        if let Some(prev_line) = self.content.get(self.cursor.row) {
                            if prev_line.len() > 1 {
                                self.cursor.col = prev_line.len() - 1;
                            } else {
                                self.cursor.col = 0
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            content: vec!["".to_string()],
            cursor: Default::default(),
            scroll: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Pos {
    x: usize,
    y: usize,
}

impl Pos {
    pub fn as_u16(&self) -> Option<(u16, u16)> {
        let x = u16::try_from(self.x).ok();
        let y = u16::try_from(self.y).ok();

        x.and_then(|r| y.map(|c| (r, c)))
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    pub fn as_u16(&self) -> Option<(u16, u16)> {
        let row = u16::try_from(self.row).ok();
        let col = u16::try_from(self.col).ok();

        row.and_then(|r| col.map(|c| (r, c)))
    }

    pub fn from_cursor_and_scroll_offset(cursor: &Cursor, pos: &Pos) -> Self {
        Cursor {
            row: cursor.row + pos.x,
            col: cursor.col + pos.y,
        }
    }
}

#[derive(Debug)]
pub struct UiMode {
    pub headline: String,
    pub body: Vec<String>,
    pub modeline: String,
    pub cursor: Cursor,
}

impl Default for UiMode {
    fn default() -> Self {
        let (columns, rows) = terminal::size().expect("could not get terminal size");

        Self {
            headline: "-".repeat(columns.into()),
            body: (2..rows).map(|_| "~".into()).collect(),
            modeline: format!("|> Normal {} <|", " ".repeat((columns - 13).into())),
            cursor: Default::default(),
        }
    }
}

impl UiMode {
    pub fn for_tui(&self) -> String {
        String::new() + &self.headline + "\n" + &self.body.join("\n") + "\n" + &self.modeline
    }

    pub fn update_body(&mut self, mut content: Vec<String>) {
        let (columns, rows) = terminal::size().expect("could not get terminal size");

        let filled_lines: u16 = content.len() as u16 + 2;

        let mut empty_rows: Vec<_> = (0..(rows - filled_lines))
            .map(|_| "~".to_string())
            .collect();

        content.append(&mut empty_rows);

        self.body = content;
    }

    pub fn update_cursor<F>(&mut self, cb: F)
    where
        F: FnOnce(&Cursor) -> Cursor,
    {
        self.cursor = cb(&self.cursor);
    }
}

#[derive(Debug)]
pub struct ModelineMode {
    left: String,
    row: usize,
    col: usize,
}

impl Default for ModelineMode {
    fn default() -> Self {
        Self {
            left: "|> Normal".into(),
            col: 0,
            row: 0,
        }
    }
}

// impl ModelineMode {
//     pub fn setup() {

//     }
// }

pub fn update_modeline(ctx: &mut Ctx, e: &AppEvent) {
    if let AppEvent::Move(x, y) = e {
        ctx.modes.modeline.col += *y as usize;
        ctx.modes.modeline.row += *x as usize;
    }
}

// impl ModelineMode {
//     pub fn setup_hooks<'a>(&'a mut self, hooks: &'a mut Hooks) {
//         hooks.on_type.push(&mut |txt: &str| {
//             self.col += txt.len();
//         })
//     }
// }

// struct Hooks<'a> {
//     on_type: Vec<&'a mut dyn FnOnce(&str)>,
// }
