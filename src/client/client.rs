use crate::client::building::BuildingMenu;
use crate::client::network_handler::NetworkHandler;
use crate::client::{event_handler, interface_handler};
use crate::game::game_data::{GameData, State};
use crate::game::world::World;
use crate::texture_handler::{TextureHandler, TextureId};
use crate::windowing::Windowing;
use crate::{constants, ticks};
use core::net::SocketAddr;
use rapier2d::math::Vector;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::ttf::Sdl2TtfContext;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Client<'ttf> {
    pub(in crate::client) game: GameData,
    pub(in crate::client) windowing: Windowing,
    pub(in crate::client) network_handler: Arc<RwLock<Option<NetworkHandler>>>,
    pub(in crate::client) texture_handler: TextureHandler<'ttf>,
    pub world: World,

    pub(in crate::client) server_address: String,
    pub(in crate::client) username: String,

    pub(in crate::client) building_menu: BuildingMenu,

    running: bool,
    pub(in crate::client) typing_ip: bool,
    pub(in crate::client) typing_username: bool,

    pub(in crate::client) scroll: i32,
    target_scroll: i32,
}

impl<'ttf> Client<'ttf> {
    pub fn new(ttf_context: &'ttf Sdl2TtfContext) -> Result<Client<'ttf>, String> {
        let mut windowing = Windowing::new(constants::window::CLIENT_TITLE)?;
        let texture_handler = TextureHandler::new(ttf_context, &mut windowing.canvas)?;

        Ok(Client {
            game: Default::default(),
            windowing,
            network_handler: Arc::new(RwLock::new(None)),
            texture_handler,
            world: World::new(),

            server_address: String::from("127.0.0.1"),
            username: String::from("Username"),

            building_menu: BuildingMenu::new(),

            running: true,
            typing_ip: false,
            typing_username: false,

            scroll: 0,
            target_scroll: 0,
        })
    }

    pub async fn run(&mut self) {
        let mut last_tick = ticks();

        *self.game.window_size.write().await = self.windowing.canvas.window().size();

        'running: while self.running {
            let now = ticks();
            let tick_time = now - last_tick;
            last_tick = now;
            let dt = (tick_time as f32 / 1000.0).min(constants::MAX_FRAME_TIME);

            for event in self.windowing.event_pump.poll_iter().collect::<Vec<_>>() {
                match event {
                    Event::Quit { .. } => break 'running,
                    Event::KeyDown {
                        keycode: Some(k), ..
                    } => self.handle_key_event(k, true).await,
                    Event::KeyUp {
                        keycode: Some(k), ..
                    } => self.handle_key_event(k, false).await,
                    _ => {}
                };

                event_handler::handle_event(self, event).await;
            }

            self.tick(dt).await;
            if let Err(e) = self.render().await {
                eprintln!("Error rendering: {e}");
            }
        }
    }

    async fn tick(&mut self, dt: f32) {
        if *self.game.state.read().await == State::BuildingMenu {
            self.building_menu.tick(dt, &self.windowing);
        }

        self.world.tick(dt);
        self.game
            .component_list
            .write()
            .await
            .items_mut()
            .iter_mut()
            .for_each(|(_, c)| c.tick(dt));

        let d_scroll = self.target_scroll - self.scroll;
        if d_scroll.abs() > 0 {
            let sign = d_scroll / d_scroll.abs();
            let mag = (d_scroll as f32 * dt * 10.0).abs().max(1.0);

            self.scroll += sign * mag as i32;
        }
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

    async fn render_game(&mut self) -> Result<(), String> {
        self.windowing.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.windowing.canvas.clear();

        self.world
            .render(&mut self.windowing.canvas, &self.texture_handler)?;

        Ok(())
    }

    fn render_building_menu(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn handle_key_event(&mut self, key: Keycode, pressed: bool) {
        if key == Keycode::SPACE && pressed {
            let gravity = self.world.gravity.y;
            self.world.gravity = Vector::new(0.0, -gravity);
        }
    }

    pub fn connect(&mut self) -> Result<(), String> {
        let game = self.game.clone();
        let network_handler = self.network_handler.clone();
        let addr = self.server_address.clone();
        let username = self.username.clone();

        tokio::spawn(async move {
            *game.state.write().await = State::Connecting;

            let addr = match format!("{}:{}", addr, constants::networking::SERVER_PORT)
                .parse::<SocketAddr>()
            {
                Ok(a) => a,
                Err(e) => {
                    *game.state.write().await = State::ConnectFailed;
                    *game.connect_err.write().await = format!("Invalid address: {}", e);
                    return;
                }
            };

            let new_network_handler = match NetworkHandler::new(game.clone(), addr, username).await
            {
                Ok(h) => h,
                Err(e) => {
                    *game.state.write().await = State::ConnectFailed;
                    *game.connect_err.write().await = format!("Failed to connect: {}", e);
                    return;
                }
            };

            *network_handler.write().await = Some(new_network_handler);
        });

        Ok(())
    }

    async fn disconnect(&mut self) {
        if let Some(network_handler) = self.network_handler.write().await.take() {
            network_handler.shutdown().await;
        }
    }

    pub fn get_windowing(&self) -> &Windowing {
        &self.windowing
    }

    pub fn get_texture_handler(&'_ self) -> &'_ TextureHandler<'_> {
        &self.texture_handler
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    fn scroll(&mut self, amount: i32) {
        self.target_scroll += amount;
        self.target_scroll = self.target_scroll.min(0);
    }

    pub(crate) fn handle_part_scrolling(
        &mut self,
        mouse_x: i32,
        amount: i32,
    ) {
        let (window_w, _window_h) = self.windowing.canvas.window().size();

        let box_tex = self.texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        if mouse_x >= x {
            self.scroll(amount * box_tex.1.1 as i32);
            return;
        }
    }
}
