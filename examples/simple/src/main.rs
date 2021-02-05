use log::info;
use rocket::{info_, log_, get, routes, response::Redirect};
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
    Redirect::to("/login?username=")
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, index_anon])
        .attach(Airlock::<hatch::SimpleHatch>::fairing())
}
