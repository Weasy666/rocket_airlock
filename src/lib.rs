// Some unused words in the context of spaceship/rocket theme and the source of
// inspiration: https://spaceflight.nasa.gov/shuttle/reference/shutref/structure/airlock.html
// - pressure chamber
// - compartment
// - bulkhead

use std::{marker::Sized, sync::Arc};
use log::info;
use rocket::{
    info_, log_, Rocket, Route, State, try_outcome,
    fairing::{AdHoc, Fairing},
    request::{FromRequest, Outcome, Request}
};
use yansi::Paint;


/// Whenever a hatch needs to cross-check information with or needs to ask for
/// permission at mission control, it uses the communicator to contact and speak with it.
#[rocket::async_trait]
pub trait Communicator: Send + Sync {
    async fn from<'a>(rocket: &'a Rocket) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}

#[rocket::async_trait]
impl Communicator for () {
    async fn from(_rocket: &Rocket) -> Result<Self, Box<dyn std::error::Error>> { Ok(()) }
}

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
    async fn from<'a>(rocket: &'a Rocket) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}

/// The security airlock is the entry point to a rocket. Everything from the outside environment
/// that wants to enter a rocket, needs to go through its hatches and pass all their security checks.
pub struct Airlock<H: Hatch> { pub hatch: Arc<H> }

impl<H: Hatch + 'static> Airlock<H> {
    pub fn fairing() -> impl Fairing {
        AdHoc::on_attach(H::name(), |rocket| async {
            let hatch = HatchBuilder::<H>::from(&rocket)
                .build()
                .await
                .expect(&format!("Error parsing config for Hatch {}", H::name()));//std::any::type_name::<K>()

            Ok(rocket.attach(Airlock::fairing_custom(hatch)))
        })
    }

    pub fn fairing_with_comm(comm: H::Comm) -> impl Fairing {
        AdHoc::on_attach(H::name(), |rocket| async {
            let hatch = HatchBuilder::<H>::from(&rocket)
                .with_comm(comm)
                .build()
                .await
                .expect(&format!("Error parsing config for Hatch {}", H::name()));

            Ok(rocket.attach(Airlock::fairing_custom(hatch)))
        })
    }

    pub fn fairing_custom(hatch: H) -> impl Fairing {
        AdHoc::on_attach(H::name(), |rocket| async {
            info_!("Installing airlock with hatch into rocket");
            Ok(rocket.manage(Arc::new(hatch))
                .mount("/", H::routes())
            )
        })
    }
}

struct HatchBuilder<'a, H: Hatch> {
    rocket: Option<&'a Rocket>,
    comm: Option<H::Comm>,
    hatch: Option<H>
}

impl<'a, H: Hatch + 'static> HatchBuilder<'a, H>{
    #[allow(dead_code)]
    fn from_hatch(hatch: H) -> Self {
        HatchBuilder {
            rocket: None,
            comm: None,
            hatch: Some(hatch)
        }
    }

    fn from(rocket: &'a Rocket) -> Self {
        HatchBuilder {
            rocket: Some(rocket),
            comm: None,
            hatch: None
        }
    }

    fn with_comm(mut self, comm: H::Comm) -> Self {
        self.comm = Some(comm);
        self
    }

    async fn build(self) -> Result<H, Box<dyn std::error::Error>> {
        let emoji = if cfg!(windows) {""} else {"🛡️ "};
        info!("{}{}", Paint::masked(emoji), Paint::magenta(format!("Airlock Hatch {}:", Paint::blue(H::name()))).wrap());

        let mut hatch = if let Some(rocket) = self.rocket {
            info_!("Loading config from Rocket.toml");
            H::from(rocket).await?
        } else {
            self.hatch.expect("A builder can only be created from a Config or another Hatch, so one should at least be present")
        };

        if let Some(comm) = self.comm {
            info_!("Connecting custom Communicator");
            hatch.connect_comm(comm);
        } else {
            let rocket = self.rocket.expect("Tried building Communicator without calling 'with_config(...)' on the builder.");
            hatch.connect_comm(<H::Comm as Communicator>::from(rocket).await?)
        }

        Ok(hatch)
    }
}

#[rocket::async_trait]
impl<'r, H: Hatch + 'static> FromRequest<'r> for Airlock<H> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let hatch = try_outcome!(request.guard::<State<Arc<H>>>().await);
        Outcome::Success(Airlock{ hatch: hatch.inner().clone() })
    }
}
