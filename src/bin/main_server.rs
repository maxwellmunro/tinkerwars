use tinkerwars::server::server::Server;

#[tokio::main]
async fn main() {
    let ttf_context = sdl2::ttf::init().unwrap();
    let mut server = Server::new(&ttf_context).await.unwrap();
    server.run().await;
}
