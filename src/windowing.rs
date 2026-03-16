use crate::constants;
use sdl2::EventPump;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct Windowing {
    pub canvas: Canvas<Window>,
    pub event_pump: EventPump,
}

impl Windowing {
    pub fn new(title: &str) -> Result<Windowing, String> {
        // sdl2::hint::set("SDL_RENDER_DRIVER", "opengl");
        // sdl2::hint::set("SDL_RENDER_VSYNC", "1");
        //
        // unsafe {
        //     std::env::set_var("SDL_VIDEODRIVER", "x11");
        // }

        let sdl_context = sdl2::init()?;

        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem
            .window(
                title,
                constants::window::DEFAULT_WIDTH,
                constants::window::DEFAULT_HEIGHT,
            )
            .position_centered()
            .resizable()
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;
        let canvas = window
            .into_canvas()
            .present_vsync()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;

        println!("Renderer: {:?}", canvas.info());

        let event_pump = sdl_context.event_pump()?;

        Ok(Windowing { canvas, event_pump })
    }
}
