use rocket_airlock::{Airlock, Hatch, Result as HatchResult};
use rocket::{
    Build, info_, Rocket, Route,
    http::{Cookie, CookieJar, SameSite, Status},
    response::Redirect,
};
use serde::Deserialize;

pub struct SimpleHatch {
    valid_user: String
}

impl SimpleHatch {
    pub fn authenticate_username(&self, username: &str) -> bool {
        // Normally you would use self.comm() to communicate with an authentication provider, or
        // you would speak with your database or something else to authenticate the user,
        // but for this example we will just assume that every user with the same name as
        // the configured valid_user is...err...valid.
        info_!("Authenticating '{}' against valid user '{}'", username, self.valid_user);
        self.valid_user == username
    }

    pub fn is_session_expired(&self, username: &str) -> bool {
        // Normally you would pass in a session struct or a JWT or something like that,
        // but for this example we will just assume that the session is stil valid.
        self.valid_user != username
    }
}

#[rocket::async_trait]
impl Hatch for SimpleHatch {
    type Comm = ();
    type Error = crate::Error;

    fn comm(&self) -> &Self::Comm { &() }

    fn name() -> &'static str {
        "Simple"
    }

    fn routes() -> Vec<Route> {
        rocket::routes![login]
    }

    async fn from(rocket: Rocket<Build>) -> HatchResult<SimpleHatch, Self::Error> {
        let name = SimpleHatch::name().replace(" ", "").to_lowercase();
        let config = match rocket.figment().extract_inner::<HatchConfig>(&format!("airlock.{}", name)) {
            Ok(config) => config,
            Err(e) => return Err((rocket, e.into())),
        };
        Ok((rocket, SimpleHatch { valid_user: config.valid_user }))
    }
}

#[derive(Debug, Deserialize)]
struct HatchConfig {
    valid_user: String
}

#[rocket::get("/login?<username>")]
pub fn login(airlock: Airlock<SimpleHatch>, username: String, cookies: &CookieJar<'_>) -> Result<Redirect, Status> {
    info_!("Someone tries to log in with username: {}", &username);
    match airlock.hatch.authenticate_username(&username) {
        true => {
            info_!("Authentication successfull!");
            cookies.add_private(
                Cookie::build(("logged_in", username))
                    .same_site(SameSite::Lax)
            );
            Ok(Redirect::to("/"))
        }
        _ =>  Err(Status::Unauthorized),
    }
}
