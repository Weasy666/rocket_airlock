use rocket::{get, info_, response::Redirect, routes};
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
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, index_anon])
        .attach(Airlock::<hatch::SimpleHatch>::fairing())
}
