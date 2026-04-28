mod shared;
mod server;
mod client;

pub fn run_server() {
    server::run();
}

pub fn run_client() {
    client::run();
}
