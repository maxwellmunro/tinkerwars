use crate::game::game_data::{GameData, State};
use crate::game::world::World;
use crate::server::network_handler::NetworkHandler;
use crate::texture_handler::TextureHandler;
use crate::windowing::Windowing;
use crate::{constants, ticks};
use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::ttf::Sdl2TtfContext;
use crate::server::{event_handler, interface_handler};

pub struct Server<'ttf> {
    pub(in crate::server) game: GameData,
    pub(in crate::server) windowing: Windowing,
    pub(in crate::server) network_handler: NetworkHandler,
    pub(in crate::server) texture_handler: TextureHandler<'ttf>,
    pub world: World,
}

impl<'ttf> Server<'ttf> {
    pub async fn new(ttf_context: &'ttf Sdl2TtfContext) -> Result<Server<'ttf>, String> {
        let game = GameData::default();
        let mut windowing = Windowing::new(constants::window::SERVER_TITLE)?;
        let network_handler = NetworkHandler::new(game.clone()).await;
        let texture_handler = TextureHandler::new(ttf_context, &mut windowing.canvas)?;

        *game.state.write().await = State::Lobby;

        Ok(Server {
            game,
            windowing,
            network_handler,
            texture_handler,
            world: World::new(),
        })
    }

    pub async fn run(&mut self) {
        let mut last_tick = ticks();

        'running: loop {
            let state = self.game.state.read().await.clone();

            let now = ticks();
            let tick_time = now - last_tick;
            last_tick = now;
            let dt = (tick_time as f32 / 1000.0).min(constants::MAX_FRAME_TIME);

            for event in self.windowing.event_pump.poll_iter().collect::<Vec<_>>() {
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => {}
                }

                event_handler::handle_event(self, state, event).await;
            }

            self.tick(dt).await;
            if let Err(e) = self.render().await {
                eprintln!("Error rendering: {e}");
            }
        }
    }

    async fn tick(&mut self, dt: f32) {
        self.world.tick(dt);
    }

    async fn render(&mut self) -> Result<(), String> {
        self.windowing
            .canvas
            .set_draw_color(Color::RGB(140, 110, 100));
        self.windowing.canvas.clear();

        interface_handler::render_menu(self).await?;

        self.windowing.canvas.present();
        Ok(())
    }
}
