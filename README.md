# rocket_airlock
> TL;DR: Authentication and Authorization for [`Rocket`] applications.

The security airlock is the entry point to a rocket. Everything from the outside environment
that wants to enter a rocket, needs to go through its hatches and pass all their security checks.

The architecture of `rocket_airlock` was heavily inspired by Jeb Rosen's [`rocket_oauth2`](https://github.com/jebrosen/rocket_oauth2) crate.

## Examples
Examples can be found in the `examples` folder. On your terminal, just navigate into the examples folder, e.g. `cd examples/simple`,
and run `cargo run` in it.

## License

As the rest of the [`Rocket`] ecosystem, `rocket_airlock` is licensed under either of the following, at your option:

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[`Rocket`]: https://github.com/SergioBenitez/Rocket/
