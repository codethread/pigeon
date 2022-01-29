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
                self.modes
                    .buff
                    .insert_self(&mut self.modes.ui, &k.to_string(), &mut self.queue);
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
            info!("getting lines start {} end {}", start, end);
            let size_request = end - start;
            let empty = size_request.saturating_sub(self.text.len_lines());

            let start_post = self.text.line_to_char(start);

            info!("empty {}", empty);
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
