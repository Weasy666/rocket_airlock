use rocket::{info_, get, routes, response::Redirect};
use rocket_airlock::Airlock;
use user::User;

mod hatch;
mod user;


#[get("/")]
fn index(user: User) -> String {
    format!("Hello user: {}", user.name)
}

#[get("/", rank = 2)]
fn index_anon() -> Redirect {
    info_!("Anonymous user requested / -> redirecting to /login");
    Redirect::to("/login")
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, index_anon])
        .attach(Airlock::<hatch::OidcHatch>::fairing())
}
