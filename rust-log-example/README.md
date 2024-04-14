# Log and Error Handling Examples
This Rust project has code used in two blog posts, one on using logging with the `log` and `log4rs` crates
and a subsequent addition showing the use of the `thiserror` crate. You can read the blog posts on Medium here:
* [Logging in a Rust Application](https://medium.com/@raysuliteanu/logging-in-a-rust-application-36afc34dcc5d)
* [Improving Your Error Handling in Rust](https://raysuliteanu.medium.com/improving-your-error-handling-in-rust-5d348a6d9286)

## Running the example
Just use `$ cargo run`. You should get a failure, due to a missing config file. This is on purpose to show the error handling.
If you want to see a different error, create an empty config file e.g. `$ touch config.yaml`.
