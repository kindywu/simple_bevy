pub mod shared;
mod server;
mod client;

pub fn run_server(api_key: &str) {
    server::run(api_key);
}

pub fn run_client() {
    client::run();
}
