use log::info;
use rocket_airlock::{Airlock, Hatch};
use rocket::{config::{Config, ConfigError}, Route, http::{Cookie, Cookies, SameSite, Status}, response::Redirect, info_, log_};

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

    fn comm(&self) -> &Self::Comm { &() }

    fn name() -> &'static str {
        "Simple"
    }

    fn routes() -> Vec<Route> {
        rocket::routes![login]
    }

    async fn from_config(config: &Config) -> Result<SimpleHatch, ConfigError> {
        let name = SimpleHatch::name().replace(" ", "").to_lowercase();
        let airlocks = config.get_table("airlock")?;
        let hatch = airlocks
            .get(&name)
            .ok_or_else(|| ConfigError::Missing(name.to_string()))?;

        let valid_user_value = hatch
            .get("valid_user")
            .ok_or_else(|| ConfigError::Missing("valid_user".to_string()))?;

        let valid_user = valid_user_value
            .as_str()
            .ok_or_else(|| ConfigError::BadType("valid_user".to_string(), "string", valid_user_value.type_str(), None))?
            .to_string();

        Ok(SimpleHatch{ valid_user })
    }
}

#[rocket::get("/login?<username>")]
pub fn login(airlock: Airlock<SimpleHatch>, username: String, mut cookies: Cookies<'_>) -> Result<Redirect, Status> {
    info_!("Someone tries to log in with username: {}", &username);
    match airlock.hatch.authenticate_username(&username) {
        true => {
            info_!("Authentication successfull!");
            cookies.add_private(
                Cookie::build("logged_in", username)
                    .same_site(SameSite::Lax)
                    .finish(),
            );
            Ok(Redirect::to("/"))
        }
        _ =>  Err(Status::Unauthorized),
    }
}
