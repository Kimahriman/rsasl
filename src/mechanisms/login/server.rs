use std::error::Error;
use std::fmt::{Display, Formatter};
use thiserror::Error;
use crate::mechanism::{Authentication, MechanismError};
use crate::session::{MechanismData, State};
use std::io::Write;
use std::ptr::NonNull;
use std::str::Utf8Error;
use crate::context::{Demand, DemandReply, Provider};
use crate::error::MechanismErrorKind;
use crate::error::SessionError::MechanismDone;
use crate::prelude::SessionError;
use crate::property::{AuthId, Password};

#[derive(Debug, Error)]
enum LoginError {
    #[error(transparent)]
    Utf8(Utf8Error)
}

impl MechanismError for LoginError {
    fn kind(&self) -> MechanismErrorKind {
        MechanismErrorKind::Parse
    }
}

#[derive(Debug)]
pub struct Login {
    state: LoginState,
}
#[derive(Debug, Eq, PartialEq)]
enum LoginState {
    New,
    WaitingForUsername,
    WaitingForPassword(String),
    Done,
}
impl Login {
    pub fn new() -> Self {
        Self {
            state: LoginState::New,
        }
    }
}
impl Authentication for Login {
    fn step(
        &mut self,
        session: &mut MechanismData,
        input: Option<&[u8]>,
        writer: &mut dyn Write,
    ) -> Result<(State, Option<usize>), SessionError> {
        match self.state {
            LoginState::New => {
                let out = b"User Name\0";
                writer.write_all(out)?;
                self.state = LoginState::WaitingForUsername;
                Ok((State::Running, Some(out.len())))
            },
            LoginState::WaitingForUsername => {
                if let Some(input) = input {
                    let username = std::str::from_utf8(input)
                        .map_err(|e| LoginError::Utf8(e))?
                        .to_string();

                    let out = b"Password\0";
                    writer.write_all(out)?;
                    self.state = LoginState::WaitingForPassword(username);
                    Ok((State::Running, Some(out.len())))
                } else {
                    Err(SessionError::InputDataRequired)
                }
            }
            LoginState::WaitingForPassword(ref username) => {
                if let Some(input) = input {
                    struct LoginProvider<'a> {
                        authid: &'a str,
                        password: &'a [u8],
                    }
                    impl Provider for LoginProvider<'_> {
                        fn provide<'a>(&'a self, req: &mut Demand<'a>) -> DemandReply<()> {
                            req
                                .provide_ref::<AuthId>(&self.authid)?
                                .provide_ref::<Password>(&self.password)?
                                .done()
                        }
                    }
                    let prov = LoginProvider {
                        authid: username.as_str(),
                        password: input,
                    };
                    session.validate(&prov)?;
                    Ok((State::Finished, None))
                } else {
                    Err(SessionError::InputDataRequired)
                }
            }
            LoginState::Done => Err(SessionError::MechanismDone),
        }
    }
}