// Some unused words in the context of spaceship/rocket theme and the source of
// inspiration: https://spaceflight.nasa.gov/shuttle/reference/shutref/structure/airlock.html
// - pressure chamber
// - compartment
// - bulkhead

use std::{convert::Infallible, marker::Sized, sync::Arc};
use rocket::{
    Build, info_, info, Rocket, Route, State,
    fairing::{AdHoc, Fairing},
    request::{FromRequest, Outcome, Request}
};
use yansi::Paint;


pub type Result<T, E> = std::result::Result<(Rocket<Build>, T), (Rocket<Build>, E)>;

/// Whenever a hatch needs to cross-check information with or needs to ask for
/// permission at mission control, it uses the communicator to contact and speak with it.
#[rocket::async_trait]
pub trait Communicator: Send + Sync {
    type Error: std::error::Error;
    async fn from(rocket: Rocket<Build>) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[rocket::async_trait]
impl Communicator for () {
    type Error = Infallible;
    async fn from(rocket: Rocket<Build>) -> Result<Self, Self::Error> { Ok((rocket, ())) }
}

/// A hatch isolates the airlock from the outside environment and only grants entry
/// after all its security checks are passed. Otherwise it remains shut and denies access.
#[rocket::async_trait]
pub trait Hatch: Send + Sync {
    /// Whenever the Hatch needs to cross-check information with or needs to ask for
    /// permission at mission control, it uses the communicator to contact and speak with it.
    /// If you don't need a chatty Hatch, then just use () as your Comm type.
    type Comm: Communicator;
    type Error: std::error::Error;

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
    async fn from(rocket: Rocket<Build>) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// The security airlock is the entry point to a rocket. Everything from the outside environment
/// that wants to enter a rocket, needs to go through its hatches and pass all their security checks.
pub struct Airlock<H: Hatch> { pub hatch: Arc<H> }

impl<H: Hatch + 'static> Airlock<H> {
    pub fn fairing() -> impl Fairing {
        AdHoc::try_on_ignite(H::name(), |rocket| async {
            let (rocket, hatch) = match HatchBuilder::<H>::from(rocket)
                .build()
                .await {
                    Ok(h) => h,
                    Err((rocket, e)) => {
                        log::error!("Error parsing config for Hatch `{}`: {:?}", H::name(), e);//std::any::type_name::<K>()
                        return Err(rocket);
                    },
                };

            Ok(Self::finish_setup(rocket, hatch))
        })
    }

    pub fn fairing_with_comm(comm: H::Comm) -> impl Fairing {
        AdHoc::try_on_ignite(H::name(), |rocket| async {
            let (rocket, hatch) = match HatchBuilder::<H>::from(rocket)
                .with_comm(comm)
                .build()
                .await {
                    Ok(h) => h,
                    Err((rocket, e)) => {
                        log::error!("Error parsing config for Hatch `{}`: {:?}", H::name(), e);//std::any::type_name::<K>()
                        return Err(rocket);
                    },
                };

            Ok(Self::finish_setup(rocket, hatch))
        })
    }

    pub fn fairing_custom(hatch: H) -> impl Fairing {
        AdHoc::try_on_ignite(H::name(), |rocket| async {
            Ok(Self::finish_setup(rocket, hatch))
        })
    }

    fn finish_setup(rocket: Rocket<Build>, hatch: H) -> Rocket<Build> {
        info_!("Installing airlock with hatch into rocket");
        rocket.manage(Arc::new(hatch))
            .mount("/", H::routes())
    }
}

struct HatchBuilder<H: Hatch> {
    rocket: Rocket<Build>,
    comm: Option<H::Comm>,
    hatch: Option<H>
}

impl<H: Hatch + 'static> HatchBuilder<H>{
    fn from(rocket: Rocket<Build>) -> Self {
        HatchBuilder {
            rocket,
            comm: None,
            hatch: None
        }
    }

    #[allow(dead_code)]
    fn with_hatch(mut self, hatch: H) -> Self {
        self.hatch = Some(hatch);
        self
    }

    fn with_comm(mut self, comm: H::Comm) -> Self {
        self.comm = Some(comm);
        self
    }

    async fn build(self) -> std::result::Result<(Rocket<Build>, H), (Rocket<Build>, Box<dyn std::error::Error>)> {
        let emoji = if cfg!(windows) {""} else {"🛡️ "};
        info!("{}{}", Paint::mask(emoji), Paint::magenta(&format!("Airlock Hatch {}:", Paint::blue(H::name()))).wrap());

        let rocket = self.rocket;
        let (rocket, mut hatch) = if let Some(hatch) = self.hatch {
            info_!("Using provided hatch: `{}`", H::name());
            (rocket, hatch)
        } else {
            info_!("Extracting config from Rocket");
            H::from(rocket).await
                .map_err(|(rocket, e)| (rocket, e.into()))?
        };

        let (rocket, comm) = if let Some(comm) = self.comm {
            info_!("Connecting custom Communicator");
            (rocket, comm)
        } else {
            <H::Comm as Communicator>::from(rocket).await
                .map_err(|(rocket, e)| (rocket, e.into()))?
        };
        hatch.connect_comm(comm);

        Ok((rocket, hatch))
    }
}

#[rocket::async_trait]
impl<'r, H: Hatch + 'static> FromRequest<'r> for Airlock<H> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<&State<Arc<H>>>().await {
            Outcome::Success(h) => Outcome::Success(Airlock {
                hatch: h.inner().clone(),
            }),
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}
