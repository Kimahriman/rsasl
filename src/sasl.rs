
use crate::channel_bindings::{ChannelBindingCallback, NoChannelBindings};
use crate::config::{ClientConfig, ClientSide, ConfigSide, SASLConfig, ServerConfig, ServerSide};

use crate::error::{SASLError};

use crate::mechname::Mechname;

use crate::registry::{Mechanism, StartFn};
use crate::session::{ClientSession, ServerSession, Session, Side};
use crate::validate::{NoValidation, Validation};

use std::marker::PhantomData;
use std::sync::Arc;
use crate::mechanism::Authentication;

#[derive(Debug)]
/// SASL Provider context
///
pub struct SASL<V: Validation = NoValidation, CB = NoChannelBindings> {
    pub(crate) config: Arc<SASLConfig>,
    pub(crate) cb: CB,
    pub(crate) validation: Option<V::Value>,
}
impl SASL {
    pub fn client(config: Arc<SASLConfig>) -> Self {
        Self {
            config,
            cb: NoChannelBindings,
            validation: None,
        }
    }
}
impl<V: Validation> SASL<V> {
    pub fn server(config: Arc<SASLConfig>) -> Self {
        Self {
            config,
            cb: NoChannelBindings,
            validation: None,
        }
    }
}

/// ### Provider functions
///
/// These methods are only available when compiled with feature `provider`
/// or `provider_base64` (enabled by default).
/// They are mainly relevant for protocol implementations wanting to start an
/// authentication exchange.
impl<CB: ChannelBindingCallback, V: Validation> SASL<V, CB> {
    pub fn with_cb(config: Arc<SASLConfig>, cb: CB) -> Self {
        Self {
            config,
            cb,
            validation: None,
        }
    }

    fn start_inner<'a, F>(
        self,
        f: F,
        offered: &[&Mechname]
    ) -> Result<Session<V, CB>, SASLError>
        where F: for<'b> Fn(&'b Mechanism) -> Option<StartFn>
    {
        let config = self.config.clone();
        offered
            .iter()
            .filter_map(|offered_mechname| {
                config.mech_list()
                    .find(|avail_mech| avail_mech.mechanism == *offered_mechname)
                    .filter(self.config.filter)
                    .and_then(|mech| {
                        let start = f(mech)?;
                        let auth = start(config.as_ref(), offered).ok()?;
                        Some((mech, auth))
                    })
            })
            .max_by(|(m,_), (n,_)| (self.config.sorter)(m, n))
            .map_or(Err(SASLError::NoSharedMechanism), |(selected, auth)| {
                Ok(Session::new(
                    self,
                    Side::Client,
                    auth,
                    selected.clone(),
                ))
            })
    }

    /// Starts a authentication exchange as a client
    ///
    /// Depending on the mechanism chosen this may need additional data from the application, e.g.
    /// an authcid, optional authzid and password for PLAIN. To provide that data an application
    /// has to either call `set_property` before running the step that requires the data, or
    /// install a callback.
    pub fn client_start_suggested<'a>(
        self,
        offered: &[&Mechname],
    ) -> Result<Session<V, CB>, SASLError>
    {
        self.start_inner(|mech| mech.client, offered)
    }

    /// Starts a authentication exchange as the server role
    ///
    /// An application acting as server will most likely need to implement a callback to check the
    /// authentication data provided by the user.
    ///
    /// See [SessionCallback] on how to implement callbacks.
    pub fn server_start_suggested<'a>(
        self,
        offered: &[&Mechname],
    ) -> Result<Session<V, CB>, SASLError>
    {
        self.start_inner(|mech| mech.server, offered)
    }
}