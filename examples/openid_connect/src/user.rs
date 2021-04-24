use crate::hatch;
use hatch::OidcHatch;
use rocket::{info_, Request, request::{FromRequest, Outcome}};
use rocket_airlock::{Airlock, Hatch};


#[derive(Debug)]
pub(crate) struct User {
    pub(crate) name: String
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        match cookies.get_private("oicd_access_token") {
            Some(token_cookie) => {
                let hatch = request.guard::<Airlock<OidcHatch>>()
                    .await
                    .expect(&format!("Hatch '{}' was not installed into the airlock.", OidcHatch::name()))
                    .hatch;

                if hatch.validate_access_token(token_cookie.value()) {
                    let username = cookies.get_private("username").unwrap().value().to_string();

                    info_!("User '{}' logged in!", &username);
                    return Outcome::Success(User{ name: username })
                }

                Outcome::Forward(())
            },
            _ => Outcome::Forward(())
        }
    }
}
