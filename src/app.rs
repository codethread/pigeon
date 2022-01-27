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
use rust_fsm::StateMachine;

use crate::{
    experiments::UserMachine,
    modes::{update_modeline, Ctx, Cursor, Modes},
    renderer::{RendResult, Renderer},
};

pub struct App {
    pub renderer: Renderer,
    pub user_machine: StateMachine<UserMachine>,
    pub modes: Modes,
    pub queue: Vec<AppEvent>,
    // cbs: HashMap<AppEvent, Vec<Box<dyn FnOnce(&mut Ctx<'_>, &AppEvent)>>>,
    pub cbs: HashMap<AppEventKey, Vec<fn(&mut Ctx<'_>, &AppEvent)>>,
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            renderer: Default::default(),
            user_machine: Default::default(),
            modes: Default::default(),
            queue: Default::default(),
            cbs: Default::default(),
        };

        {
            // XXX: small hack just to get a first line in

            let first_line = app.modes.ui.body.get_mut(0).unwrap();
            let buffer_line = &app
                .modes
                .buff
                .buffers
                .get(0)
                .expect("buffer should exist")
                .content;

            let buffer_line = buffer_line.get(0).expect("buffer should have text");

            *first_line = buffer_line.clone();

            app.modes.ui.update_cursor(|Cursor { col, row }| Cursor {
                row: *row,
                col: col + buffer_line.len(),
            });
        }

        app.cbs.insert(
            AppEventKey::from(AppEvent::Move(0, 0)),
            vec![update_modeline],
        );

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

        let ui = self.modes.ui.for_tui();

        self.renderer.draw(ui)?;
        self.renderer.cursor(&self.modes.ui.cursor)?;

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

        self.renderer.draw(ui)?;
        self.renderer.cursor(&self.modes.ui.cursor)?;

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
