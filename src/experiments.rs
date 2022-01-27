use rust_fsm::StateMachineImpl;

#[derive(Debug)]
pub enum UserInput {
    Key(char),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UserState {
    Idle,
    Prompt,
}

#[derive(Debug)]
pub enum UserOutput {
    None,
    Choice(String),
}

#[derive(Debug)]
pub struct UserMachine;

impl StateMachineImpl for UserMachine {
    type Input = UserInput;

    type State = UserState;

    type Output = UserOutput;

    const INITIAL_STATE: Self::State = UserState::Idle;

    fn transition(state: &Self::State, input: &Self::Input) -> Option<Self::State> {
        match (state, input) {
            (UserState::Idle, UserInput::Key(k)) => {
                if *k == 'p' {
                    Some(UserState::Prompt)
                } else {
                    Some(UserState::Idle)
                }
            }
            (UserState::Prompt, UserInput::Key(k)) => {
                if let 'a' | 'b' | 'c' = *k {
                    Some(UserState::Idle)
                } else {
                    Some(UserState::Prompt)
                }
            }
        }
    }

    fn output(state: &Self::State, input: &Self::Input) -> Option<Self::Output> {
        let res = match (state, input) {
            (UserState::Idle, UserInput::Key(_)) => return None,
            (UserState::Prompt, UserInput::Key(k)) => match k {
                'a' => UserOutput::Choice("cat".into()),
                'b' => UserOutput::Choice("dog".into()),
                'c' => UserOutput::Choice("fish".into()),
                _ => return None,
            },
        };

        Some(res)
    }
}
