use std::convert::Infallible;

use rocket::{Build, Rocket};

/// Whenever a hatch needs to cross-check information with or needs to ask for
/// permission at mission control, it uses the communicator to contact and speak with it.
#[rocket::async_trait]
pub trait Communicator: Send + Sync {
    type Error: std::error::Error;
    async fn from(rocket: Rocket<Build>) -> crate::Result<Self, Self::Error>
    where
        Self: Sized;
}

#[rocket::async_trait]
impl Communicator for () {
    type Error = Infallible;
    async fn from(rocket: Rocket<Build>) -> crate::Result<Self, Self::Error> {
        Ok((rocket, ()))
    }
}
