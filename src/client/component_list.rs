use crate::game::component::{ComponentKind, render_component};
use crate::texture_handler::TextureHandler;
use crate::{constants, ticks};
use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::HashMap;
use std::f32::consts::PI;

#[derive(Clone, Default)]
pub(crate) struct ComponentListSet {
    items: HashMap<u64, ComponentListItem>,
    next_id: u64,
}

impl ComponentListSet {
    pub fn insert(&mut self, item: ComponentListItem) {
        self.items.insert(self.next_id, item);
        self.next_id += 1;
    }

    pub fn remove(&mut self, id: u64) {
        self.items.remove(&id);
    }

    pub fn items(&self) -> &HashMap<u64, ComponentListItem> {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut HashMap<u64, ComponentListItem> {
        &mut self.items
    }
}

#[derive(Clone)]
pub(crate) struct ComponentListItem {
    init_distance: f32,

    start_x: f32,
    start_y: f32,
    start_rot: f32,

    spawn_time: u64,

    t: f32,

    target_rot: f32,
    target_x: f32,
    target_y: f32,

    kind: ComponentKind,
    count: u64,
}

impl ComponentListItem {
    pub(in crate::client) fn new(tx: f32, ty: f32, kind: ComponentKind, count: u64) -> Self {
        let dx = 0.5 - tx;
        let dy = 0.5 - ty;

        Self {
            init_distance: (dx * dx + dy * dy).sqrt(),

            start_x: 0.5,
            start_y: 0.5,
            start_rot: rand::thread_rng().gen_range(0.0..(2.0 * PI)),

            spawn_time: ticks(),

            t: 0.0,

            target_x: tx,
            target_y: ty,
            target_rot: rand::thread_rng().gen_range(0.0..(2.0 * PI)),

            kind,
            count,
        }
    }
    pub(in crate::client) fn render(
        &self,
        canvas: &mut Canvas<Window>,
        texture_handler: &TextureHandler,
    ) -> Result<(), String> {
        let (window_w, window_h) = canvas.window().size();

        render_component(
            canvas,
            texture_handler,
            self.kind,
            (self.cur_x() * window_w as f32) as i32,
            (self.cur_y() * window_h as f32) as i32,
            self.cur_rot(),
            1.0,
            false,
        )?;

        let text_tex = texture_handler.render_text(
            &format!("x{}", self.count),
            canvas,
            Color::RGB(255, 255, 255),
        )?;

        let text_rect = Rect::new(
            (self.cur_x() * window_w as f32) as i32,
            (self.cur_y() * window_h as f32) as i32,
            text_tex.1.0,
            text_tex.1.1,
        );

        canvas.copy(&text_tex.0, None, Some(text_rect))?;

        Ok(())
    }

    pub(in crate::client) fn tick(&mut self, dt: f32) {
        if self.t >= 1.0 {
            return;
        }

        let dt = ticks() - self.spawn_time;

        let total_time = self.init_distance * constants::PART_SELECT_MS_PER_UNIT as f32;

        let t = (dt as f32 / total_time).min(1.0);

        //     -cos(PI * t) - 1
        // t = ----------------
        //            2
        let t = -((PI * t).cos() - 1.0) / 2.0;

        self.t = t.min(1.0);
    }

    pub(in crate::client) fn cur_x(&self) -> f32 {
        self.t * self.target_x + (1.0 - self.t) * self.start_x
    }

    pub(in crate::client) fn cur_y(&self) -> f32 {
        self.t * self.target_y + (1.0 - self.t) * self.start_y
    }

    pub(in crate::client) fn cur_rot(&self) -> f32 {
        self.t * self.target_rot + (1.0 - self.t) * self.start_rot
    }

    pub(in crate::client) fn kind(&self) -> ComponentKind {
        self.kind
    }

    pub(in crate::client) fn count(&self) -> u64 {
        self.count
    }

    pub(crate) fn target_x(&self) -> f32 {
        self.target_x
    }

    pub(crate) fn target_y(&self) -> f32 {
        self.target_y
    }
}
