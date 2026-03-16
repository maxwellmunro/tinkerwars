use tokio::runtime::Handle;
use tokio::task::yield_now;
use tinkerwars::client::client::Client;
use tinkerwars::ticks;

#[tokio::main]
async fn main() {
    // tokio::spawn(async move {
    //    let mut last_print = ticks();
    //
    //     loop {
    //         while ticks() - last_print < 1000 {
    //             yield_now().await;
    //         }
    //
    //         last_print = ticks();
    //
    //         println!("Thread count: {}", Handle::current().metrics().num_alive_tasks());
    //     }
    // });

    let ttf_context = sdl2::ttf::init().unwrap();
    let mut client = Client::new(&ttf_context).unwrap();
    client.run().await;
}
