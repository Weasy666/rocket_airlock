// Some unused words in the context of spaceship/rocket theme and the source of
// inspiration: https://spaceflight.nasa.gov/shuttle/reference/shutref/structure/airlock.html
// - pressure chamber
// - compartment
// - bulkhead

use log::info;
use rocket::{config::{Config, ConfigError}, fairing::{AdHoc, Fairing}, info_, log_, request::{FromRequest, Outcome, Request}, Route};
use std::{marker::Sized, sync::Arc};
use yansi::Paint;


/// Whenever a hatch needs to cross-check information with or needs to ask for
/// permission at mission control, it uses the communicator to contact and speak with it.
#[rocket::async_trait]
pub trait Communicator: Send + Sync + 'static {}

impl Communicator for () {}

/// A hatch isolates the airlock from the outside environment and only grants entry
/// after all its security checks are passed. Otherwise it remains shut and denies access.
#[rocket::async_trait]
pub trait Hatch: Send + Sync {
    /// Whenever the Hatch needs to cross-check information with or needs to ask for
    /// permission at mission control, it uses the communicator to contact and speak with it.
    /// If you don't need a chatty Hatch, then just use () as your Comm type.
    type Comm: Communicator;

    /// This is like an intercom, press the button and speak into it, or in this case, call
    /// the function and us the `Comm` to speak to your mission control.
    fn comm(&self) -> &Self::Comm;

    /// Used to connect a Communicator to a Hatch. The Communicator should be stored in some way,
    /// so that a Hatch can use it for all communications with its mission control.
    #[allow(unused_variables)]
    fn connect_comm(&mut self, comm: Self::Comm) {}

    /// Name of the Hatch. Is used for the Fairing name and also for log messages during its installation.
    fn name() -> &'static str;

    /// The Routes a Hatch is going to mount. If a Hatch does not need to mount any Routes, then this
    /// function can be ignored, as the standard implementation will then return an empty vector.
    fn routes() -> Vec<Route> { Vec::new() }

    /// With this function a Hatch can be created and configured with parameters that are present in
    /// rockets config file. It is async so you can fully configure your hatch, even if you need to
    /// do some delaying task, such as discovering an OpenID Connect manifest at a remote provider.
    async fn from_config(config: &Config) -> Result<Self, ConfigError>
    where
        Self: Sized;
}

/// The security airlock is the entry point to a rocket. Everything from the outside environment
/// that wants to enter a rocket, needs to go through its hatches and pass all their security checks.
pub struct Airlock<H: Hatch> { pub hatch: Arc<H> }

impl<H: Hatch + 'static> Airlock<H> {
    pub fn fairing() -> impl Fairing {
        Self::fairing_with_comm(None)
    }

    pub fn fairing_with_comm(comm: Option<H::Comm>) -> impl Fairing {
        AdHoc::on_attach(H::name(), |mut rocket| async {
            let emoji = if cfg!(windows) {""} else {"🛡️ "};
            info!("{}{}", Paint::masked(emoji), Paint::magenta(format!("Airlock Hatch {}:", Paint::blue(H::name()))).wrap());

            info_!("Loading config from Rocket.toml");
            let config = rocket.config().await;
            let mut hatch = H::from_config(config).await
                .expect(&format!("Missing config for Hatch {}", H::name()));
            if let Some(comm) = comm {
                info_!("Connecting custom Communicator");
                hatch.connect_comm(comm);
            }

            info_!("Installing airlock with hatch into rocket");
            Ok(rocket.manage(Arc::new(hatch))
                .mount("/", H::routes())
            )
        })
    }
}

#[rocket::async_trait]
impl<'a, 'r, H: Hatch + 'static> FromRequest<'a, 'r> for Airlock<H> {
    type Error = ();

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let hatch = request
            .managed_state::<Arc<H>>()
            .expect("This type of hatch was not installed into the airlock.");
        Outcome::Success(Airlock{ hatch: hatch.clone() })
    }
}
