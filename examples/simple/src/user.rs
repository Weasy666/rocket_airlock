use rocket::{ http::Status, request::{FromRequest, Outcome}, Request};
use rocket_airlock::Airlock;
use crate::hatch;


#[derive(Debug)]
pub(crate) struct User {
    pub(crate) name: String
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        match cookies.get_private("logged_in") {
            Some(logged_in) => {
                let username = logged_in.value().to_string();
                // Here you could do something else with your hatch, like checking session lifetime or other stuff.
                let hatch = request.guard::<Airlock<hatch::SimpleHatch>>()
                    .await
                    .expect("Hatch 'SimpleHatch' was not installed into the airlock.")
                    .hatch;

                if hatch.is_session_expired(&username) {
                    // If session is expired, forward user to the next route, which in this case is /login.
                    return Outcome::Forward(Status::Ok);
                }

                Outcome::Success(User{ name: username })
            },
            _ => Outcome::Forward(Status::Ok)
        }
    }
}
