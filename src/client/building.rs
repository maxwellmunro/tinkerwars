use crate::client::programming::{Command, CommandData, CommandHandle, CommandSet, render_command};
use crate::constants::get_command_shape;
use crate::game::component::{ComponentKind, render_component};
use crate::game::game_data::{GameData, State};
use crate::polygon::Vec2;
use crate::texture_handler::{TextureHandler, TextureId};
use crate::windowing::Windowing;
use crate::{constants, polygon};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use std::collections::HashMap;
use std::f32::consts::PI;

#[derive(Clone)]
pub struct RobotComponent {
    kind: ComponentKind,
    x: f32,
    y: f32,

    /// Radians
    rot: f32,
}

impl RobotComponent {
    /// list of hitboxes transformed to world coordinates + rotation
    fn get_shapes(&self) -> Vec<Vec<rapier2d::math::Point<f32>>> {
        constants::get_component_shape(self.kind)
            .to_vec()
            .into_iter()
            .map(|s| {
                let mut shape = s.to_vec();
                polygon::rotate_polygon(&mut shape, self.rot);
                polygon::translate_polygon(&mut shape, self.x, self.y);
                shape
            })
            .collect::<Vec<_>>()
    }

    pub fn kind(&self) -> ComponentKind {
        self.kind
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn rot(&self) -> f32 {
        self.rot
    }
}

#[derive(Default, Clone)]
pub(crate) struct Screw {
    /// (comp_id, body_id)
    a: (u64, u64),

    /// (comp_id, body_id)
    b: (u64, u64),
    pos: Vec2,
}

impl Screw {
    pub(crate) fn a(&self) -> (u64, u64) {
        self.a
    }

    pub(crate) fn b(&self) -> (u64, u64) {
        self.b
    }

    pub(crate) fn pos(&self) -> Vec2 {
        self.pos
    }
}

#[derive(Default, Clone)]
pub(crate) struct Robot {
    components: HashMap<u64, RobotComponent>,
    commands: CommandSet,
    pub screws: Vec<Screw>,
    next_id: u64,
}

impl Robot {
    pub fn new() -> Self {
        Robot {
            components: HashMap::new(),
            commands: CommandSet::new(),
            screws: Vec::new(),
            next_id: 0,
        }
    }

    pub fn add_component(&mut self, comp: RobotComponent) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.components.insert(id, comp);
        id
    }

    pub fn add_command(&mut self, command: Command) -> CommandHandle {
        self.commands.add_command(command)
    }

    /// Remove component + add any connected screws back to `GameData::building_components`
    pub async fn remove_component(
        &mut self,
        id: u64,
        game_data: &GameData,
    ) -> Option<RobotComponent> {
        let pre = self.screws.len();

        self.screws.retain(|s| s.a.0 != id && s.b.0 != id);

        let pst = self.screws.len();

        let screw_count = pst - pre;

        if screw_count > 0 {
            let mut building_components = game_data.building_components.write().await;
            let count = building_components.get_mut(&ComponentKind::Screw);
            if let Some(count) = count {
                *count += screw_count as u64;
            } else {
                building_components.insert(ComponentKind::Screw, screw_count as u64);
            }
        }

        self.components.remove(&id)
    }

    pub(crate) fn components(&self) -> &HashMap<u64, RobotComponent> {
        &self.components
    }
}

enum ScrewSelectingState {
    SelectingA,
    SelectingB,
}

#[derive(Debug, Default, Clone, Copy)]
enum BuildingState {
    #[default]
    Assembling,
    Programming,
}

pub(in crate::client) struct BuildingMenu {
    /// Current scroll in px
    pub(in crate::client) scroll: i32,

    /// Target scroll in px
    pub(in crate::client) target_scroll: i32,

    pan_button_down_x: f32,
    pan_button_down_y: f32,
    pan_button_down: bool,

    shift_down: bool,

    /// Camera offset x in world coordinates
    dx: f32,

    /// Camera offset y in world coordinates
    dy: f32,
    zoom: f32,

    robot: Robot,

    /// Part kind current being dragged
    part_selected: Option<ComponentKind>,

    /// Rotation of part currently being dragged
    part_rot: f32,

    render_hitboxes: bool,

    selected_id: Option<u64>,

    screwing_selections: Vec<(u64, u64)>,
    screw_part_a: Option<(u64, u64)>,
    screw_part_b: Option<(u64, u64)>,
    screw_world_pos: Vec2,

    /// Current index of `screwing_selections`
    selecting_screw_id: Option<u64>,
    screw_selecting_state: ScrewSelectingState,

    state: BuildingState,
}

impl BuildingMenu {
    pub(in crate::client) fn new() -> Self {
        BuildingMenu {
            scroll: 0,
            target_scroll: 0,

            pan_button_down_x: 0.0,
            pan_button_down_y: 0.0,
            pan_button_down: false,

            shift_down: false,

            dx: 0.0,
            dy: 0.0,
            zoom: 1.0,

            robot: Robot::new(),

            part_selected: None,
            part_rot: 0.0,

            render_hitboxes: true,

            selected_id: None,

            screwing_selections: Vec::new(),

            screw_part_a: None,
            screw_part_b: None,

            screw_world_pos: Vec2 { x: 0.0, y: 0.0 },

            selecting_screw_id: None,
            screw_selecting_state: ScrewSelectingState::SelectingA,

            state: Default::default(),
        }
    }

    pub(in crate::client) fn tick(&mut self, dt: f32) {
        let d_scroll = self.target_scroll - self.scroll;
        if d_scroll.abs() > 0 {
            let sign = d_scroll / d_scroll.abs();
            let mag = (d_scroll as f32 * dt * 10.0).abs().max(1.0);

            self.scroll += sign * mag as i32;
        }
    }

    pub(in crate::client) async fn render(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler<'_>,
        game_data: GameData,
    ) -> Result<(), String> {
        match self.state {
            BuildingState::Assembling => {
                self.render_robot(windowing, texture_handler)?;

                if self.render_hitboxes {
                    self.render_hitboxes(windowing)?;
                }

                if self.selecting_screw_id.is_none() {
                    self.render_part_select(game_data, windowing, texture_handler)
                        .await?;
                    self.render_dragging_part(windowing, texture_handler)?;

                    let done_tex = texture_handler.get_texture(TextureId::DoneButton);

                    let rect = Rect::new(10, 10, done_tex.1.0, done_tex.1.1);

                    windowing.canvas.copy(done_tex.0, None, Some(rect))?;

                    let state_tex = texture_handler.get_texture(TextureId::BuildingStateButton);

                    let src = Rect::new(0, 0, 63, 25);
                    let dst = Rect::new(0, 20 + rect.height() as i32, src.width(), src.height());
                    windowing.canvas.copy(state_tex.0, Some(src), Some(dst))?;
                }
            }
            BuildingState::Programming => {
                self.render_commands(windowing, texture_handler)?;

                if self.render_hitboxes {
                    self.render_command_hitboxes(windowing)?;
                }

                let done_size = texture_handler.get_texture(TextureId::DoneButton).1;

                let state_tex = texture_handler.get_texture(TextureId::BuildingStateButton);

                let src = Rect::new(0, 25, 63, 25);
                let dst = Rect::new(10, 20 + done_size.1 as i32, state_tex.1.0, state_tex.1.1);
                windowing.canvas.copy(state_tex.0, Some(src), Some(dst))?;
            }
        }

        Ok(())
    }

    pub async fn handle_event(
        &mut self,
        windowing: &Windowing,
        texture_handler: &TextureHandler<'_>,
        game_data: GameData,
        event: Event,
    ) {
        if !matches!(event, Event::KeyDown { .. }) && self.selecting_screw_id.is_some() {
            return;
        }

        match event {
            Event::MouseButtonDown {
                x, y, mouse_btn, ..
            } => {
                if mouse_btn == MouseButton::Right {
                    self.handle_pan(x, y, true);
                }

                if mouse_btn == MouseButton::Middle {
                    self.robot.commands.add_command(Command::new(Vec2 { x: 0.0, y: 0.0 }, CommandData::If));
                }

                if mouse_btn == MouseButton::Left {
                    self.handle_part_drag(x, y, &game_data, windowing, texture_handler)
                        .await;
                    self.handle_part_select(x, y);
                    self.handle_ui(x, y, game_data, texture_handler).await;
                }
            }
            Event::MouseButtonUp {
                x, y, mouse_btn, ..
            } => {
                if mouse_btn == MouseButton::Right {
                    self.handle_pan(x, y, false);
                }

                if mouse_btn == MouseButton::Left {
                    self.handle_part_drop(x, y, game_data, windowing, texture_handler)
                        .await;
                }
            }
            Event::MouseWheel {
                mouse_x,
                mouse_y,
                y,
                ..
            } => {
                self.handle_part_scrolling(mouse_x, y, windowing, texture_handler);
                self.handle_zoom(mouse_x, mouse_y, y, windowing, texture_handler);
            }
            Event::KeyDown {
                keycode: Some(key), ..
            } => self.handle_key_event(game_data, key, true).await,
            Event::KeyUp {
                keycode: Some(key), ..
            } => self.handle_key_event(game_data, key, false).await,
            _ => {}
        }
    }

    fn render_robot(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler,
    ) -> Result<(), String> {
        let (mouse_x, mouse_y) = {
            let pos = windowing.event_pump.mouse_state();
            (pos.x(), pos.y())
        };

        self.robot.components.iter().try_for_each(|(id, comp)| {
            let (mut world_x, mut world_y) = (comp.x, comp.y);

            if self.pan_button_down {
                let (mouse_world_x, mouse_world_y) = self.to_world(mouse_x, mouse_y);

                world_x += mouse_world_x - self.pan_button_down_x;
                world_y += mouse_world_y - self.pan_button_down_y;
            }

            let (x, y) = self.to_screen(world_x, world_y);

            let selected = if let Some(selected) = self.selected_id
                && selected == *id
            {
                true
            } else if let Some(screw_part_a) = self.screw_part_a
                && screw_part_a.0 == *id
            {
                true
            } else if let Some(screw_part_b) = self.screw_part_b
                && screw_part_b.0 == *id
            {
                true
            } else {
                false
            };

            render_component(
                &mut windowing.canvas,
                texture_handler,
                comp.kind,
                x,
                y,
                comp.rot,
                self.zoom,
                selected,
            )?;

            Ok::<(), String>(())
        })?;

        self.robot.screws.iter().try_for_each(|s| {
            let (mut world_x, mut world_y) = (s.pos.x, s.pos.y);

            if self.pan_button_down {
                let (mouse_world_x, mouse_world_y) = self.to_world(mouse_x, mouse_y);

                world_x += mouse_world_x - self.pan_button_down_x;
                world_y += mouse_world_y - self.pan_button_down_y;
            }

            let (x, y) = self.to_screen(world_x, world_y);

            render_component(
                &mut windowing.canvas,
                texture_handler,
                ComponentKind::Screw,
                x,
                y,
                0.0,
                self.zoom,
                false,
            )?;

            Ok::<(), String>(())
        })
    }

    fn render_commands(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler,
    ) -> Result<(), String> {
        let (mouse_x, mouse_y) = {
            let pos = windowing.event_pump.mouse_state();
            (pos.x(), pos.y())
        };

        self.robot.commands.get_commands().iter().try_for_each(|(_id, command)| {
            let (mut world_x, mut world_y) = command.get_pos();

            if self.pan_button_down {
                let (mouse_world_x, mouse_world_y) = self.to_world(mouse_x, mouse_y);

                world_x += mouse_world_x - self.pan_button_down_x;
                world_y += mouse_world_y - self.pan_button_down_y;
            }

            let (x, y) = self.to_screen(world_x, world_y);

//            let selected = if let Some(selected) = self.selected_id
//                && selected == *id
//            {
//                true
//            } else if let Some(screw_part_a) = self.screw_part_a
//                && screw_part_a.0 == *id
//            {
//                true
//            } else if let Some(screw_part_b) = self.screw_part_b
//                && screw_part_b.0 == *id
//            {
//                true
//            } else {
//                false
//            };

            render_command(
                &mut windowing.canvas,
                texture_handler,
                command.get_data(),
                x,
                y,
                0.0,
                self.zoom,
                false,
            )?;

            Ok::<(), String>(())
        })?;

        Ok(())
    }

    async fn render_part_select(
        &self,
        game_data: GameData,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler<'_>,
    ) -> Result<(), String> {
        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        game_data
            .building_components
            .read()
            .await
            .iter()
            .enumerate()
            .try_for_each(|(i, (kind, count))| {
                let y = i as i32 * box_tex.1.1 as i32 + self.scroll;

                let rect = Rect::new(x, y, box_tex.1.1, box_tex.1.1);
                windowing.canvas.copy(&box_tex.0, None, Some(rect))?;

                let tex = texture_handler.get_texture(constants::get_component_icon_texture(*kind));
                let rect = Rect::new(
                    x + (box_tex.1.0 as i32 - tex.1.0 as i32) / 2,
                    y + (box_tex.1.1 as i32 - tex.1.1 as i32) / 2,
                    tex.1.0,
                    tex.1.1,
                );

                windowing.canvas.copy(&tex.0, None, Some(rect))?;

                let tex = texture_handler.render_text(
                    &format!("x{}", count),
                    &mut windowing.canvas,
                    Color::RGB(255, 255, 255),
                )?;
                let rect = Rect::new(
                    window_w as i32 - tex.1.0 as i32 - 10,
                    y + box_tex.1.1 as i32 - tex.1.1 as i32 - 10,
                    tex.1.0,
                    tex.1.1,
                );
                windowing.canvas.copy(&tex.0, None, Some(rect))?;

                Ok::<(), String>(())
            })
    }

    fn render_dragging_part(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler,
    ) -> Result<(), String> {
        if let Some(kind) = self.part_selected {
            let mouse_state = windowing.event_pump.mouse_state();
            render_component(
                &mut windowing.canvas,
                texture_handler,
                kind,
                mouse_state.x(),
                mouse_state.y(),
                self.part_rot,
                self.zoom,
                false,
            )?;
        }

        Ok(())
    }

    fn render_command_hitboxes(&self, windowing: &mut Windowing) -> Result<(), String> {
        let comps = self
            .robot
            .commands
            .get_commands()
            .iter()
            .map(|(_, command)| get_command_shape(command.get_data()))
            .flatten()
            .collect::<Vec<_>>();

        windowing.canvas.set_draw_color(Color::RGB(255, 0, 0));

        let mouse_state = windowing.event_pump.mouse_state();
        let (mouse_x, mouse_y) = (mouse_state.x(), mouse_state.y());

        comps.into_iter().try_for_each(|shape| {
            for w in shape.windows(2) {
                let mut a = (w[0].x, w[0].y);
                let mut b = (w[1].x, w[1].y);

                if self.pan_button_down {
                    let (mouse_world_x, mouse_world_y) = self.to_world(mouse_x, mouse_y);

                    a.0 += mouse_world_x - self.pan_button_down_x;
                    a.1 += mouse_world_y - self.pan_button_down_y;

                    b.0 += mouse_world_x - self.pan_button_down_x;
                    b.1 += mouse_world_y - self.pan_button_down_y;
                }

                let (mut x1, mut y1) = self.to_screen(a.0, a.1);
                let (mut x2, mut y2) = self.to_screen(b.0, b.1);

                x1 /= constants::PIXELS_PER_METER as i32;
                x2 /= constants::PIXELS_PER_METER as i32;
                y1 /= constants::PIXELS_PER_METER as i32;
                y2 /= constants::PIXELS_PER_METER as i32;

                let a = Point::new(x1 as i32, y1 as i32);
                let b = Point::new(x2 as i32, y2 as i32);

                windowing.canvas.draw_line(a, b)?;
            }

            if shape.len() >= 2 {
                let (x1, y1) = self.to_screen(shape[0].x, shape[0].y);
                let (x2, y2) = self.to_screen(shape.last().unwrap().x, shape.last().unwrap().y);

                let a = Point::new(x1 as i32, y1 as i32);
                let b = Point::new(x2 as i32, y2 as i32);

                windowing.canvas.draw_line(a, b)?;
            }
            Ok::<(), String>(())
        })
    }

    fn render_hitboxes(&self, windowing: &mut Windowing) -> Result<(), String> {
        let comps = self
            .robot
            .components
            .iter()
            .map(|(_, comp)| comp.get_shapes())
            .flatten()
            .collect::<Vec<_>>();

        windowing.canvas.set_draw_color(Color::RGB(255, 0, 0));

        let mouse_state = windowing.event_pump.mouse_state();
        let (mouse_x, mouse_y) = (mouse_state.x(), mouse_state.y());

        comps.into_iter().try_for_each(|shape| {
            for w in shape.windows(2) {
                let mut a = (w[0].x, w[0].y);
                let mut b = (w[1].x, w[1].y);

                if self.pan_button_down {
                    let (mouse_world_x, mouse_world_y) = self.to_world(mouse_x, mouse_y);

                    a.0 += mouse_world_x - self.pan_button_down_x;
                    a.1 += mouse_world_y - self.pan_button_down_y;

                    b.0 += mouse_world_x - self.pan_button_down_x;
                    b.1 += mouse_world_y - self.pan_button_down_y;
                }

                let (x1, y1) = self.to_screen(a.0, a.1);
                let (x2, y2) = self.to_screen(b.0, b.1);

                let a = Point::new(x1 as i32, y1 as i32);
                let b = Point::new(x2 as i32, y2 as i32);

                windowing.canvas.draw_line(a, b)?;
            }

            if shape.len() >= 2 {
                let (x1, y1) = self.to_screen(shape[0].x, shape[0].y);
                let (x2, y2) = self.to_screen(shape.last().unwrap().x, shape.last().unwrap().y);

                let a = Point::new(x1 as i32, y1 as i32);
                let b = Point::new(x2 as i32, y2 as i32);

                windowing.canvas.draw_line(a, b)?;
            }
            Ok::<(), String>(())
        })
    }

    async fn handle_key_event(&mut self, game_data: GameData, key: Keycode, pressed: bool) {
        if key == Keycode::LShift {
            self.shift_down = pressed;
        }

        if let Some(mut id) = self.selecting_screw_id {
            if !pressed {
                return;
            }

            if key == Keycode::Right {
                id += 1;
                if id as usize >= self.screwing_selections.len() {
                    id = 0;
                }
            }

            if key == Keycode::Left {
                if id == 0 {
                    id = self.screwing_selections.len() as u64;
                }

                id -= 1;
            }

            self.selecting_screw_id = Some(id);

            match self.screw_selecting_state {
                ScrewSelectingState::SelectingA => {
                    self.screw_part_a = Some(self.screwing_selections[id as usize]);

                    if key == Keycode::Return {
                        self.screw_selecting_state = ScrewSelectingState::SelectingB;
                        self.screwing_selections.remove(id as usize);
                        self.screw_part_b = Some(self.screwing_selections[0]);
                    }
                }
                ScrewSelectingState::SelectingB => {
                    self.screw_part_b = Some(self.screwing_selections[id as usize]);
                    if key == Keycode::Return {
                        self.screw_selecting_state = ScrewSelectingState::SelectingA;
                        self.robot.screws.push(Screw {
                            a: self.screw_part_a.unwrap(),
                            b: self.screw_part_b.unwrap(),
                            pos: self.screw_world_pos,
                        });

                        self.screw_part_a = None;
                        self.screw_part_b = None;

                        self.selecting_screw_id = None;

                        self.screwing_selections.clear();
                    }
                }
            }

            return;
        }

        if pressed && let Some(id) = self.selected_id {
            if key != Keycode::DELETE {
                return;
            }

            let Some(comp) = self.robot.remove_component(id, &game_data).await else {
                return;
            };

            let mut building_components = game_data.building_components.write().await;

            let found = building_components.iter_mut().any(|(kind, count)| {
                if *kind == comp.kind {
                    *count += 1;
                    return true;
                }

                false
            });

            if !found {
                building_components.insert(comp.kind, 1);
            }
        }
    }

    fn handle_pan(&mut self, mouse_x: i32, mouse_y: i32, pressed: bool) {
        self.pan_button_down = pressed;

        if !pressed {
            let (world_x, world_y) = self.to_world(mouse_x, mouse_y);

            self.dx -= world_x - self.pan_button_down_x;
            self.dy -= world_y - self.pan_button_down_y;
        } else {
            (self.pan_button_down_x, self.pan_button_down_y) = self.to_world(mouse_x, mouse_y);
        }
    }

    async fn handle_part_drag(
        &mut self,
        mouse_x: i32,
        mouse_y: i32,
        game_data: &GameData,
        windowing: &Windowing,
        texture_handler: &TextureHandler<'_>,
    ) {
        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        if mouse_x > x {
            let mut to_remove = vec![];

            let mut building_components = game_data.building_components.write().await;

            building_components
                .iter_mut()
                .enumerate()
                .for_each(|(i, (kind, count))| {
                    if *count == 0 {
                        to_remove.push(*kind);
                        return;
                    }

                    let y = i as i32 * box_tex.1.1 as i32 + self.scroll;

                    let rect = Rect::new(x, y, box_tex.1.0, box_tex.1.1);

                    if rect.contains_point(Point::new(mouse_x, mouse_y)) {
                        self.part_selected = Some(*kind);
                        *count -= 1;

                        if *count == 0 {
                            to_remove.push(*kind);
                        }
                    }
                });

            to_remove.into_iter().for_each(|kind| {
                building_components.remove(&kind);
            });
        }
    }

    fn handle_part_select(&mut self, mouse_x: i32, mouse_y: i32) {
        let (x, y) = self.to_world(mouse_x, mouse_y);
        let mouse_pos = rapier2d::math::Point::new(x, y);

        self.selected_id = None;

        for (id, comp) in self.robot.components.iter() {
            let selected = comp
                .get_shapes()
                .into_iter()
                .any(|s| polygon::point_intersects_polygon(mouse_pos, &s));

            if selected {
                self.selected_id = Some(*id);
                break;
            }
        }
    }

    async fn handle_ui(
        &mut self,
        mouse_x: i32,
        mouse_y: i32,
        game_data: GameData,
        texture_handler: &TextureHandler<'_>,
    ) {
        let ml = Point::new(mouse_x, mouse_y);

        let done_size = texture_handler.get_texture(TextureId::DoneButton).1;
        let rect = Rect::new(10, 10, done_size.0, done_size.1);
        if rect.contains_point(ml) {
            *game_data.state.write().await = State::InGame;
        }

        let state_size = texture_handler
            .get_texture(TextureId::BuildingStateButton)
            .1;
        let rect = Rect::new(10, 20 + rect.height() as i32, state_size.0, state_size.1);
        if rect.contains_point(ml) {
            self.state = match self.state {
                BuildingState::Assembling => BuildingState::Programming,
                BuildingState::Programming => BuildingState::Assembling,
            }
        }
    }

    async fn handle_part_drop(
        &mut self,
        mouse_x: i32,
        mouse_y: i32,
        game_data: GameData,
        windowing: &Windowing,
        texture_handler: &TextureHandler<'_>,
    ) {
        let Some(part_selected) = self.part_selected.take() else {
            return;
        };

        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        if mouse_x > x {
            let mut building_comps = game_data.building_components.write().await;

            let found = building_comps.iter_mut().any(|(kind, count)| {
                if *kind == part_selected {
                    *count += 1;
                    return true;
                }
                false
            });

            if !found {
                building_comps.insert(part_selected, 1);
            }
        } else {
            let (x, y) = self.to_world(mouse_x, mouse_y);
            if part_selected == ComponentKind::Screw {
                let point = rapier2d::math::Point::new(x, y);

                let comps = self
                    .robot
                    .components
                    .iter()
                    .map(|(id, comp)| {
                        comp.get_shapes()
                            .into_iter()
                            .enumerate()
                            .filter(|(_, s)| polygon::point_intersects_polygon(point, s))
                            .map(|(body_id, _)| (*id, body_id as u64))
                            .collect::<Vec<_>>()
                    })
                    .flatten()
                    .collect::<Vec<_>>();

                if comps.len() <= 1 {
                    for (kind, count) in game_data.building_components.write().await.iter_mut() {
                        if *kind == ComponentKind::Screw {
                            *count += 1;
                            break;
                        }
                    }

                    return;
                }

                if comps.len() == 2 {
                    self.robot.screws.push(Screw {
                        a: comps[0],
                        b: comps[1],
                        pos: Vec2 { x, y },
                    });

                    return;
                }

                self.screw_world_pos = Vec2 { x, y };
                self.screwing_selections = comps;
                self.screw_part_a = Some(self.screwing_selections[0]);
                self.selecting_screw_id = Some(0);
            } else {
                self.robot.add_component(RobotComponent {
                    kind: part_selected,
                    x,
                    y,
                    rot: self.part_rot,
                });

                self.part_rot = 0.0;
            }
        }
    }

    fn handle_part_scrolling(
        &mut self,
        mouse_x: i32,
        amount: i32,
        windowing: &Windowing,
        texture_handler: &TextureHandler<'_>,
    ) {
        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        if mouse_x >= x {
            self.scroll(amount * box_tex.1.1 as i32);
            return;
        }
    }

    fn handle_zoom(
        &mut self,
        mouse_x: i32,
        mouse_y: i32,
        amount: i32,
        windowing: &Windowing,
        texture_handler: &TextureHandler,
    ) {
        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        if mouse_x >= x {
            return;
        }

        if self.part_selected.is_some() {
            self.part_rot -= amount as f32 * PI / 4.0 / if self.shift_down { 4.0 } else { 1.0 };
            return;
        }

        let pre = self.to_world(mouse_x, mouse_y);

        let mut zoom = (self.zoom * 10.0) as i32;
        zoom += if amount > 0 { 5 } else { -5 };
        if zoom <= 0 {
            zoom = 5;
        }
        self.zoom = zoom as f32 / 10.0;

        let pst = self.to_world(mouse_x, mouse_y);

        self.dx -= pst.0 - pre.0;
        self.dy -= pst.1 - pre.1;
    }

    fn scroll(&mut self, amount: i32) {
        self.target_scroll += amount;
        self.target_scroll = self.target_scroll.min(0);
    }

    pub(in crate::client) fn to_world(&self, x: i32, y: i32) -> (f32, f32) {
        (
            x as f32 / self.zoom / constants::PIXELS_PER_METER + self.dx,
            y as f32 / self.zoom / constants::PIXELS_PER_METER + self.dy,
        )
    }

    pub(in crate::client) fn to_screen(&self, x: f32, y: f32) -> (i32, i32) {
        (
            ((x - self.dx) * self.zoom * constants::PIXELS_PER_METER) as i32,
            ((y - self.dy) * self.zoom * constants::PIXELS_PER_METER) as i32,
        )
    }
}
