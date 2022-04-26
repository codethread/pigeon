use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::{app::App, logger::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    init_logger();

    info!("app starting");

    let mut app = App::default();

    let foo = vec![1, 2, 3];

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

mod app {
    #[derive(Clone)]
    struct TextChange {
        text: String,
        buffer_id: usize,
    }

    // events
    // const E_MOVE: &'static str = "move";
    // const E_TEXT_CHANGE: &'static str = "text_change";

    #[derive(Eq, PartialEq, Hash)]
    pub struct AppEventKey(u8);

    impl From<AppEvent> for AppEventKey {
        fn from(e: AppEvent) -> Self {
            match e {
                AppEvent::TextChange(_) => AppEventKey(0),
                AppEvent::Move(_, _) => AppEventKey(1),
            }
        }
    }

    #[derive(Clone)]
    pub enum AppEvent {
        TextChange(TextChange),
        Move(i16, i16),
    }

    use std::collections::HashMap;

    use crossterm::event::KeyCode;
    use log::info;
    use rust_fsm::StateMachine;

    use crate::{
        experiments::UserMachine,
        modes::{update_modeline, Ctx, Cursor, Modes},
        renderer::{RendResult, Renderer, UI},
    };

    use self::buffer::Buffer;

    pub struct App {
        pub renderer: Renderer,
        pub ui: UI,
        pub user_machine: StateMachine<UserMachine>,
        pub buffers: Vec<Buffer>,
        pub modes: Modes,
        pub queue: Vec<AppEvent>,
        // cbs: HashMap<AppEvent, Vec<Box<dyn FnOnce(&mut Ctx<'_>, &AppEvent)>>>,
        pub cbs: HashMap<AppEventKey, Vec<fn(&mut Ctx<'_>, &AppEvent)>>,
    }

    impl Default for App {
        fn default() -> Self {
            info!("creating app");
            let mut app = Self {
                renderer: Default::default(),
                buffers: Default::default(),
                user_machine: Default::default(),
                modes: Default::default(),
                queue: Default::default(),
                cbs: Default::default(),
                ui: Default::default(),
            };

            info!("created app: {:?}", &app);

            {
                let buff = Buffer::build()
                    .with_text("scratch buffer".to_string())
                    .with_window(0)
                    .create();

                app.ui.get_active_window().set_buffer(0, &buff);

                app.buffers.push(buff);
            }

            // {
            //     // XXX: small hack just to get a first line in

            //     let first_line = app.modes.ui.body.get_mut(0).unwrap();
            //     let buffer_line = &app
            //         .modes
            //         .buff
            //         .buffers
            //         .get(0)
            //         .expect("buffer should exist")
            //         .content;

            //     let buffer_line = buffer_line.get(0).expect("buffer should have text");

            //     *first_line = buffer_line.clone();

            //     app.modes.ui.update_cursor(|Cursor { col, row }| Cursor {
            //         row: *row,
            //         col: col + buffer_line.len(),
            //     });
            // }

            // app.cbs.insert(
            //     AppEventKey::from(AppEvent::Move(0, 0)),
            //     vec![update_modeline],
            // );

            app
        }
    }

    impl App {
        pub fn handle(&mut self, code: KeyCode) -> RendResult {
            // app is Idle / Prompt
            // match (
            //     self.user_machine.consume(&UserInput::Key(k)),
            //     self.user_machine.state(),
            // ) {
            //     (Ok(Some(UserOutput::Choice(choice))), UserState::Idle) => {
            //         self.renderer.paint(&choice)?;
            //     }
            //     (Ok(_), UserState::Prompt) => {
            //         self.renderer.prompt()?;
            //     }
            //     _ => self.renderer.paint(&k)?,
            // }
            match code {
                KeyCode::Backspace => {
                    self.modes.buff.backspace(&mut self.modes.ui);
                }
                KeyCode::Delete => todo!(),
                KeyCode::Enter => {
                    self.modes.buff.insert_newline(&mut self.modes.ui);
                }
                KeyCode::Left => todo!(),
                KeyCode::Right => todo!(),
                KeyCode::Up => todo!(),
                KeyCode::Down => todo!(),
                KeyCode::Char(k) => {
                    self.modes.buff.insert_self(
                        &mut self.modes.ui,
                        &k.to_string(),
                        &mut self.queue,
                    );
                }
                _ => (),
            }

            // let ui = self.modes.ui.for_tui();

            // self.renderer.draw(ui)?;
            // self.renderer.cursor(&self.modes.ui.cursor)?;
            let screen = self.ui.screen();
            self.renderer.render(screen)?;

            Ok(())
        }

        pub fn idle(&mut self) -> RendResult {
            let mut events = Vec::new();
            events.append(&mut self.queue);

            let mut ctx = Ctx {
                modes: &mut self.modes,
            };

            for e in events {
                if let Some(cbs) = self.cbs.get(&e.clone().into()) {
                    for cb in cbs {
                        cb(&mut ctx, &e);
                    }
                }
            }

            let ui = self.modes.ui.for_tui();

            // self.renderer.draw(ui)?;
            // self.renderer.cursor(&self.modes.ui.cursor)?;
            // self.renderer.render()?;

            Ok(())
        }
    }

    impl std::fmt::Debug for App {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("App")
                .field("renderer", &self.renderer)
                .field("user_machine", &self.user_machine.state())
                .finish()
        }
    }

    pub mod buffer {
        use log::info;
        use ropey::{Rope, RopeSlice};

        use crate::modes::Cursor;

        pub struct Lines<'a> {
            pub lines: RopeSlice<'a>,
            pub empty: usize,
        }

        #[derive(Default, Debug)]
        pub struct Buffer {
            text: Rope,

            /// structure tree-sitter document, will do later
            doc: Option<i32>,

            /// reference to window ID if buffer is visible
            window: Option<usize>,

            cursor: Cursor,
        }

        impl Buffer {
            pub fn new(text: Rope, doc: Option<i32>, window: Option<usize>) -> Self {
                Self {
                    text,
                    doc,
                    window,
                    cursor: Default::default(),
                }
            }

            pub fn build() -> BufferBuilder {
                BufferBuilder::default()
            }

            pub fn open_in_window(&mut self, win_id: usize) {
                // TODO: event to tell old window of change?
                self.window = Some(win_id);
            }

            pub fn remove_from_window(&mut self) {
                // TODO: event to tell old window of change?
                self.window = None;
            }

            pub fn get_cursor(&self) -> &Cursor {
                &self.cursor
            }

            pub fn get_lines_range(&self, start: usize, end: usize) -> Lines {
                let size_request = end - start;
                let empty = size_request.saturating_sub(self.text.len_lines());

                let start_post = self.text.line_to_char(start);

                let end_pos = if empty == 0 {
                    Some(self.text.line_to_char(end + 1))
                } else {
                    None
                };

                let slice = match end_pos {
                    Some(end_pos) => self.text.slice(start_post..end_pos),
                    None => self.text.slice(start_post..),
                };

                Lines {
                    lines: slice,
                    empty,
                }
            }
        }

        #[derive(Default, Debug)]
        pub struct BufferBuilder {
            text: Option<Rope>,
            doc: Option<i32>,
            window: Option<usize>,
        }

        impl BufferBuilder {
            /// panics if values not met
            pub fn create(self) -> Buffer {
                match self {
                    BufferBuilder {
                        text: Some(text),
                        doc,
                        window,
                    } => Buffer::new(text, doc, window),
                    _ => panic!("Buffer constraints not met"),
                }
            }

            pub fn with_text(mut self, text: String) -> Self {
                self.text = Some(Rope::from(text));
                self
            }

            pub fn with_window(mut self, win: usize) -> Self {
                self.window = Some(win);
                self
            }
        }
    }
}

mod renderer {
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
}

mod experiments;

mod modes {

    use crossterm::terminal;

    use self::core::CoreMode;
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
        pub core: CoreMode,
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

    mod core {
        #[derive(Debug, Default)]
        pub struct CoreMode {}

        impl CoreMode {
            // pub fn insert_self()
        }
    }
}

mod logger;
