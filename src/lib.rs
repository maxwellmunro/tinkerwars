use sdl2::sys::SDL_GetTicks64;
pub fn ticks() -> u64 {
    unsafe { SDL_GetTicks64() as u64 }
}

pub mod client {
    pub mod client;
    mod event_handler;
    mod interface_handler;
    mod network_handler;
    mod packet_handler;
    pub(crate) mod building;
    pub(crate) mod component_list;
    pub(crate) mod programming;
}

pub mod server {
    pub mod server;
    mod network_handler;
    mod packet_handler;
    mod interface_handler;
    mod event_handler;
}

mod game {
    pub(crate) mod component;
    pub(crate) mod game_data;
    mod player;
    mod screw_link;
    pub(crate) mod world;
}

mod constants;
mod packet;
mod texture_handler;
mod windowing;
mod polygon;
