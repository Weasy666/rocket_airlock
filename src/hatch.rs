use rocket::{yansi::Paint, Build, Rocket, Route};

use crate::Communicator;

/// A hatch isolates the airlock from the outside environment and only grants entry
/// after all its security checks are passed. Otherwise it remains shut and denies access.
#[rocket::async_trait]
pub trait Hatch: Send + Sync {
    /// Whenever the Hatch needs to cross-check information with or needs to ask for
    /// permission at mission control, it uses the communicator to contact and speak with it.
    /// If you don't need a chatty Hatch, then just use () as your Comm type.
    type Comm: Communicator;
    type Error: std::error::Error;

    /// Name of the Hatch. Is used for the Fairing name and also for log messages during its installation.
    const NAME: &'static str;

    /// This is like an intercom, press the button and speak into it, or in this case, call
    /// the function and us the `Comm` to speak to your mission control.
    fn comm(&self) -> &Self::Comm;

    /// Used to connect a Communicator to a Hatch. The Communicator should be stored in some way,
    /// so that a Hatch can use it for all communications with its mission control.
    #[allow(unused_variables)]
    fn connect_comm(&mut self, comm: Self::Comm) {}

    /// The Routes a Hatch is going to mount. If a Hatch does not need to mount any Routes, then this
    /// function can be ignored, as the standard implementation will then return an empty vector.
    fn routes() -> Vec<Route> {
        Vec::new()
    }

    /// With this function a Hatch can be created and configured with parameters that are present in
    /// rockets config file. It is async so you can fully configure your hatch, even if you need to
    /// do some delaying task, such as discovering an OpenID Connect manifest at a remote provider.
    async fn from(rocket: Rocket<Build>) -> crate::Result<Self, Self::Error>
    where
        Self: Sized;
}

pub(crate) struct HatchBuilder<H: Hatch> {
    rocket: Rocket<Build>,
    comm: Option<H::Comm>,
    hatch: Option<H>,
}

impl<H: Hatch + 'static> HatchBuilder<H> {
    pub(crate) fn from(rocket: Rocket<Build>) -> Self {
        HatchBuilder {
            rocket,
            comm: None,
            hatch: None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_hatch(mut self, hatch: H) -> Self {
        self.hatch = Some(hatch);
        self
    }

    pub(crate) fn with_comm(mut self, comm: H::Comm) -> Self {
        self.comm = Some(comm);
        self
    }

    pub(crate) async fn build(
        self,
    ) -> std::result::Result<(Rocket<Build>, H), (Rocket<Build>, Box<dyn std::error::Error>)> {
        let emoji = if cfg!(windows) { "" } else { "🛡️  " };
        let end = H::NAME.rfind(" hatch");
        let name = if let Some(end) = end { &H::NAME[..end] } else { H::NAME };

        #[cfg(feature = "log")]
        log::info!(
            "{}{}",
            emoji.mask(),
            Paint::magenta(&format!("Airlock Hatch {}:", Paint::blue(name))).wrap()
        );
        #[cfg(feature = "trace")]
        tracing::info!(
            "{}{} ({}: {})",
            emoji.mask(),
            "airlock".bold().blue(),
            "hatch".bold().blue(),
            name,
        );

        let rocket = self.rocket;
        let (rocket, mut hatch) = if let Some(hatch) = self.hatch {
            let msg = "Using provided hatch";
            #[cfg(feature = "log")]
            log::info!("\r   >> {msg}");
            #[cfg(feature = "trace")]
            tracing::info!("\r   {} {}", ">>".primary(), msg.blue(),);
            (rocket, hatch)
        } else {
            let msg = "Extracting config from Rocket";
            #[cfg(feature = "log")]
            log::info!("\r   >> {msg}");
            #[cfg(feature = "trace")]
            tracing::info!("\r   {} {}", ">>".primary(), msg.blue(),);
            H::from(rocket)
                .await
                .map_err(|(rocket, e)| (rocket, e.into()))?
        };

        let (rocket, comm) = if let Some(comm) = self.comm {
            let msg = "Connecting custom Communicator";
            #[cfg(feature = "log")]
            log::info!("\r   >> {msg}");
            #[cfg(feature = "trace")]
            tracing::info!("\r   {} {}", ">>".primary(), msg.blue(),);
            (rocket, comm)
        } else {
            <H::Comm as Communicator>::from(rocket)
                .await
                .map_err(|(rocket, e)| (rocket, e.into()))?
        };
        hatch.connect_comm(comm);

        Ok((rocket, hatch))
    }
}
