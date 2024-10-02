// Some unused words in the context of spaceship/rocket theme and the source of
// inspiration: https://spaceflight.nasa.gov/shuttle/reference/shutref/structure/airlock.html
// - pressure chamber
// - compartment
// - bulkhead

use rocket::{
    fairing::{AdHoc, Fairing},
    request::{FromRequest, Outcome, Request},
    Build, Rocket, State,
};

#[cfg(feature = "trace")]
use rocket::yansi::Paint;

use std::sync::Arc;

pub use communicator::Communicator;
pub use hatch::Hatch;

mod communicator;
mod hatch;

pub type Result<T, E> = std::result::Result<(Rocket<Build>, T), (Rocket<Build>, E)>;

/// The security airlock is the entry point to a rocket. Everything from the outside environment
/// that wants to enter a rocket, needs to go through its hatches and pass all their security checks.
pub struct Airlock<H: Hatch> {
    pub hatch: Arc<H>,
}

impl<H: Hatch + 'static> Airlock<H> {
    pub fn fairing() -> impl Fairing {
        AdHoc::try_on_ignite(H::NAME, |rocket| async {
            let (rocket, hatch) = match hatch::HatchBuilder::<H>::from(rocket).build().await {
                Ok(h) => h,
                Err((rocket, e)) => {
                    #[cfg(feature = "log")]
                    log::error!("Error parsing config for Hatch `{}`: {:?}", H::NAME, e);
                    #[cfg(feature = "trace")]
                    tracing::error!("Error parsing config for Hatch `{}`: {:?}", H::NAME, e);
                    return Err(rocket);
                }
            };

            Ok(Self::finish_setup(rocket, hatch))
        })
    }

    pub fn fairing_with_comm(comm: H::Comm) -> impl Fairing {
        AdHoc::try_on_ignite(H::NAME, |rocket| async {
            let (rocket, hatch) = match hatch::HatchBuilder::<H>::from(rocket)
                .with_comm(comm)
                .build()
                .await
            {
                Ok(h) => h,
                Err((rocket, e)) => {
                    #[cfg(feature = "log")]
                    log::error!("Error parsing config for Hatch `{}`: {:?}", H::NAME, e);
                    #[cfg(feature = "trace")]
                    tracing::error!("Error parsing config for Hatch `{}`: {:?}", H::NAME, e);
                    return Err(rocket);
                }
            };

            Ok(Self::finish_setup(rocket, hatch))
        })
    }

    pub fn fairing_custom(hatch: H) -> impl Fairing {
        AdHoc::try_on_ignite(H::NAME, |rocket| async {
            Ok(Self::finish_setup(rocket, hatch))
        })
    }

    fn finish_setup(rocket: Rocket<Build>, hatch: H) -> Rocket<Build> {
        #[cfg(feature = "log")]
        log::info!("Installing airlock with hatch into rocket");
        #[cfg(feature = "trace")]
        tracing::info!(
            "\r   {} {}",
            ">>".primary(),
            "Installing airlock with hatch into rocket".blue(),
        );
        rocket.manage(Arc::new(hatch)).mount("/", H::routes())
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
