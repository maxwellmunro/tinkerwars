use crate::client::programming::{Command, CommandData, CommandHandle, CommandSet, render_command};
use crate::constants::{get_command_link_positions, get_command_shape};
use crate::game::component::{ComponentHandle, ComponentKind, render_component};
use crate::game::game_data::{GameData, State};
use crate::polygon::Vec2;
use crate::texture_handler::{TextureHandler, TextureId, destroy};
use crate::windowing::Windowing;
use crate::{constants, polygon};
use rapier2d::na::point;
use rapier2d::prelude::nalgebra;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::{MouseButton, MouseState};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::ClippingRect;
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

        let screw_count = pst as i32 - pre as i32;

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

    pub(in crate::client) fn get_command(&self, handle: CommandHandle) -> Option<&Command> {
        self.commands.get(handle)
    }

    pub(in crate::client) fn get_command_mut(
        &mut self,
        handle: CommandHandle,
    ) -> Option<&mut Command> {
        self.commands.get_mut(handle)
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

    last_world_mouse_pos: (f32, f32),

    /// Part/command king current being dragged
    part_selected: Option<ComponentKind>,
    command_selected: Option<CommandData>,

    /// Rotation of part currently being dragged
    part_rot: f32,

    render_hitboxes: bool,

    selected_component_id: Option<u64>,
    selected_command_id: Option<u64>,

    screwing_selections: Vec<(u64, u64)>,
    screw_part_a: Option<(u64, u64)>,
    screw_part_b: Option<(u64, u64)>,
    screw_world_pos: Vec2,

    /// Current index of `screwing_selections`
    selecting_screw_id: Option<u64>,
    screw_selecting_state: ScrewSelectingState,

    state: BuildingState,

    link_selected: Option<(CommandHandle, u8)>,

    left_down: bool,
    right_down: bool,

    editing_command_settings: bool,
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

            last_world_mouse_pos: (0.0, 0.0),

            part_selected: None,
            command_selected: None,

            part_rot: 0.0,

            render_hitboxes: true,

            selected_component_id: None,
            selected_command_id: None,

            screwing_selections: Vec::new(),

            screw_part_a: None,
            screw_part_b: None,

            screw_world_pos: Vec2 { x: 0.0, y: 0.0 },

            selecting_screw_id: None,
            screw_selecting_state: ScrewSelectingState::SelectingA,

            state: Default::default(),

            link_selected: None,

            left_down: false,
            right_down: false,

            editing_command_settings: false,
        }
    }

    pub(in crate::client) fn tick(&mut self, dt: f32, windowing: &Windowing) {
        let d_scroll = self.target_scroll - self.scroll;
        if d_scroll.abs() > 0 {
            let sign = d_scroll / d_scroll.abs();
            let mag = (d_scroll as f32 * dt * 10.0).abs().max(1.0);

            self.scroll += sign * mag as i32;
        }

        let state = MouseState::new(&windowing.event_pump);
        if state.x() < 300 || state.x() > windowing.canvas.window().size().0 as i32 - 200 {
            return;
        }

        let (x, y) = self.to_world(state.x(), state.y());

        if self.left_down {
            if let Some(id) = self.selected_command_id {
                if let Some(command) = self.robot.get_command_mut(CommandHandle(id)) {
                    command.move_to(x, y);
                }
            }
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
                }

                if let Some(component_id) = self.selected_component_id {
                    self.render_component_settings(windowing, texture_handler, component_id)?;
                }
            }
            BuildingState::Programming => {
                self.render_commands(windowing, texture_handler)?;

                if self.render_hitboxes {
                    self.render_command_hitboxes(windowing)?;
                }

                self.render_commands_list(windowing, texture_handler)?;
                self.render_dragging_command(windowing, texture_handler)?;

                if let Some(command_id) = self.selected_command_id {
                    self.render_command_settings(windowing, texture_handler, command_id)?;
                }
            }
        }

        let state_rect = self.get_state_rect(texture_handler);
        let state_tex = texture_handler.get_texture(TextureId::BuildingStateButton);
        let state_src = Rect::new(
            0,
            match self.state {
                BuildingState::Assembling => 0,
                BuildingState::Programming => state_tex.1.1 / constants::TEXTURE_SCALE / 2,
            } as i32,
            state_tex.1.0 / constants::TEXTURE_SCALE,
            state_tex.1.1 / constants::TEXTURE_SCALE / 2,
        );
        let done_tex = texture_handler.get_texture(TextureId::DoneButton);

        let done_rect = Rect::new(
            state_rect.x(),
            state_rect.y() + state_rect.height() as i32 + 10,
            done_tex.1.0,
            done_tex.1.1,
        );

        windowing
            .canvas
            .copy(state_tex.0, Some(state_src), Some(state_rect))?;
        windowing.canvas.copy(done_tex.0, None, Some(done_rect))?;

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
                    self.right_down = true;
                }

                if mouse_btn == MouseButton::Middle {
                    self.handle_pan(x, y, true);
                }

                if mouse_btn == MouseButton::Left {
                    self.left_down = true;

                    match self.state {
                        BuildingState::Assembling => {
                            self.handle_part_drag(x, y, &game_data, windowing, texture_handler)
                                .await;
                            self.handle_component_select(x, y);
                        }
                        BuildingState::Programming => {
                            self.handle_command_drag(x, y, windowing, texture_handler);
                            self.handle_command_select(x, y);
                            self.handle_command_link_start(x, y);
                        }
                    }

                    self.handle_ui(x, y, game_data, texture_handler).await;
                }

                if matches!(self.state, BuildingState::Programming) {
                    self.handle_command_settings(x, y, mouse_btn);
                }
            }
            Event::MouseButtonUp {
                x, y, mouse_btn, ..
            } => {
                if mouse_btn == MouseButton::Right {
                    self.right_down = false;
                } else if mouse_btn == MouseButton::Left {
                    self.left_down = false;
                } else if mouse_btn == MouseButton::Middle {
                    self.handle_pan(x, y, false);
                }

                match self.state {
                    BuildingState::Assembling => {
                        if mouse_btn == MouseButton::Left {
                            self.handle_part_drop(x, y, game_data, windowing, texture_handler)
                                .await;
                        }
                    }
                    BuildingState::Programming => {
                        self.handle_command_drop(x, y, windowing, texture_handler);
                        self.handle_command_link_end(x, y);
                    }
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
            Event::MouseMotion { x, y, .. } => match self.state {
                BuildingState::Assembling => {}
                BuildingState::Programming => {
                    let (w_x, w_y) = self.to_world(x, y);
                    if self.right_down {
                        let cutting_line = &[
                            point![w_x, w_y],
                            point![self.last_world_mouse_pos.0, self.last_world_mouse_pos.1],
                        ];

                        let handles = self
                            .robot
                            .commands
                            .get_commands()
                            .iter()
                            .map(|(o_handle, command)| {
                                command
                                    .get_outputs()
                                    .iter()
                                    .enumerate()
                                    .map(|(o_id, inputs)| {
                                        inputs
                                            .iter()
                                            .enumerate()
                                            .map(|(i_id, (i_handle, input))| {
                                                (
                                                    *o_handle,
                                                    o_id.clone(),
                                                    *i_handle,
                                                    i_id.clone(),
                                                    *input,
                                                )
                                                    .clone()
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .flatten()
                                    .collect::<Vec<_>>()
                            })
                            .flatten()
                            .filter(|(o_handle, o_id, i_handle, _, input)| {
                                let Some(o_command) =
                                    self.robot.commands.get(CommandHandle(*o_handle))
                                else {
                                    return false;
                                };
                                let Some(i_command) = self.robot.commands.get(*i_handle) else {
                                    return false;
                                };

                                let output =
                                    get_command_link_positions(o_command.get_data())[1][*o_id];
                                let input = get_command_link_positions(i_command.get_data())[0]
                                    [*input as usize];

                                let (o_x, o_y) = (
                                    output.x + o_command.get_pos().0,
                                    output.y + o_command.get_pos().1,
                                );
                                let (i_x, i_y) = (
                                    input.x + i_command.get_pos().0,
                                    input.y + i_command.get_pos().1,
                                );

                                let a = &[point![o_x, o_y], point![(o_x + i_x) / 2.0, o_y]];
                                let b = &[
                                    point![(o_x + i_x) / 2.0, o_y],
                                    point![(o_x + i_x) / 2.0, i_y],
                                ];
                                let c = &[point![(o_x + i_x) / 2.0, i_y], point![i_x, i_y]];

                                polygon::lines_intersect(cutting_line, a)
                                    || polygon::lines_intersect(cutting_line, b)
                                    || polygon::lines_intersect(cutting_line, c)
                            })
                            .collect::<Vec<_>>();

                        handles
                            .into_iter()
                            .for_each(|(o_handle, o_id, _, i_id, _)| {
                                if let Some(command) =
                                    self.robot.commands.get_mut(CommandHandle(o_handle))
                                {
                                    command.disconnect(o_id as u64, i_id as u64);
                                }
                            });
                    }
                    self.last_world_mouse_pos = (w_x, w_y);
                }
            },
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
            let (world_x, world_y) = (comp.x, comp.y);

            let (x, y) = self.to_screen(world_x, world_y, (mouse_x, mouse_y));

            let selected = if let Some(selected) = self.selected_component_id
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
            let (world_x, world_y) = (s.pos.x, s.pos.y);

            let (x, y) = self.to_screen(world_x, world_y, (mouse_x, mouse_y));

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

        self.robot
            .commands
            .get_commands()
            .iter()
            .try_for_each(|(id, command)| {
                let selected = if let Some(selected_id) = self.selected_command_id
                    && *id == selected_id
                {
                    true
                } else {
                    false
                };
                let (world_x, world_y) = command.get_pos();

                let (x, y) = self.to_screen(world_x, world_y, (mouse_x, mouse_y));

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

                command.render(
                    &mut windowing.canvas,
                    texture_handler,
                    self.zoom,
                    self,
                    selected,
                    (mouse_x, mouse_y),
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

                destroy(tex);

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

    fn render_commands_list(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler,
    ) -> Result<(), String> {
        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        constants::COMMANDS_SET
            .iter()
            .enumerate()
            .try_for_each(|(i, data)| {
                let y = i as i32 * box_tex.1.1 as i32 + self.scroll;

                let rect = Rect::new(x, y, box_tex.1.1, box_tex.1.1);
                windowing.canvas.copy(&box_tex.0, None, Some(rect))?;

                let tex = texture_handler.get_texture(constants::get_command_texture(data));
                let rect = Rect::new(
                    x + (box_tex.1.0 as i32 - tex.1.0 as i32) / 2,
                    y + (box_tex.1.1 as i32 - tex.1.1 as i32) / 2,
                    tex.1.0,
                    tex.1.1,
                );

                windowing.canvas.copy(&tex.0, None, Some(rect))?;

                Ok::<(), String>(())
            })?;
        Ok(())
    }

    fn render_dragging_command(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler,
    ) -> Result<(), String> {
        if let Some(data) = self.command_selected.as_ref() {
            let mouse_state = windowing.event_pump.mouse_state();
            render_command(
                &mut windowing.canvas,
                texture_handler,
                data,
                mouse_state.x(),
                mouse_state.y(),
                self.part_rot,
                self.zoom,
                false,
            )?;
        }

        Ok(())
    }

    fn render_command_settings(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler,
        command_id: u64,
    ) -> Result<(), String> {
        let Some(command) = self.robot.get_command(CommandHandle(command_id)) else {
            return Ok(());
        };

        let background_tex = texture_handler.get_texture(TextureId::PartSettingsBackground);

        for y in 0..=windowing.canvas.window().size().1 / background_tex.1.1 {
            let rect = Rect::new(
                0,
                (y * background_tex.1.1) as i32,
                background_tex.1.0,
                background_tex.1.1,
            );
            windowing.canvas.copy(background_tex.0, None, Some(rect))?;
        }

        let side_bar_width = background_tex.1.0 as i32 - 10;

        let clip = windowing.canvas.clip_rect();
        windowing.canvas.set_clip_rect(ClippingRect::Some(Rect::new(
            0,
            0,
            background_tex.1.0 - 10,
            windowing.canvas.window().size().1,
        )));

        match command.get_data() {
            CommandData::OnKeyDown { key } | CommandData::OnKeyUp { key } => {
                let tex = texture_handler.render_text(
                    &format!(
                        "Key: {}",
                        if self.editing_command_settings {
                            "Listening...".to_string()
                        } else {
                            key.name()
                        }
                    ),
                    &mut windowing.canvas,
                    constants::TEXT_COLOR,
                )?;

                let rect = Rect::new(
                    if self.editing_command_settings {
                        10.min(
                            side_bar_width
                                - (tex.1.0 * constants::COMMAND_SETTINGS_TEXT_SCALE) as i32,
                        )
                    } else {
                        10
                    },
                    10,
                    tex.1.0 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                    tex.1.1 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                );

                windowing.canvas.copy(&tex.0, None, Some(rect))?;

                destroy(tex);
            }
            CommandData::SetState {
                comp,
                state,
                comp_str,
            } => {
                let comp_tex = texture_handler.render_text(
                    &format!("ID: {}", comp_str),
                    &mut windowing.canvas,
                    constants::TEXT_COLOR,
                )?;

                let state_tex = texture_handler.render_text(
                    &state.to_string(),
                    &mut windowing.canvas,
                    constants::TEXT_COLOR,
                )?;

                let comp_rect = Rect::new(
                    if self.editing_command_settings {
                        10.min(
                            side_bar_width
                                - (comp_tex.1.0 * constants::COMMAND_SETTINGS_TEXT_SCALE) as i32,
                        )
                    } else {
                        10
                    },
                    10,
                    comp_tex.1.0 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                    comp_tex.1.1 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                );
                let state_rect = Rect::new(
                    10,
                    comp_rect.height() as i32 + 20,
                    state_tex.1.0 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                    state_tex.1.1 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                );

                windowing.canvas.copy(&comp_tex.0, None, Some(comp_rect))?;
                windowing
                    .canvas
                    .copy(&state_tex.0, None, Some(state_rect))?;

                destroy(comp_tex);
                destroy(state_tex);
            }
            CommandData::Const { val_str, .. } => {
                let tex = texture_handler.render_text(
                    &format!("Val: {}", val_str),
                    &mut windowing.canvas,
                    constants::TEXT_COLOR,
                )?;

                let rect = Rect::new(
                    if self.editing_command_settings {
                        10.min(
                            side_bar_width
                                - (tex.1.0 * constants::COMMAND_SETTINGS_TEXT_SCALE) as i32,
                        )
                    } else {
                        10
                    },
                    10,
                    tex.1.0 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                    tex.1.1 * constants::COMMAND_SETTINGS_TEXT_SCALE,
                );

                windowing.canvas.copy(&tex.0, None, Some(rect))?;
                destroy(tex);
            }
            _ => {}
        }

        windowing.canvas.set_clip_rect(clip);

        Ok(())
    }

    fn render_component_settings(
        &self,
        windowing: &mut Windowing,
        texture_handler: &TextureHandler,
        component_id: u64,
    ) -> Result<(), String> {
        let Some(component) = self.robot.components.get(&component_id) else {
            return Ok(());
        };

        let background_tex = texture_handler.get_texture(TextureId::PartSettingsBackground);

        for y in 0..=windowing.canvas.window().size().1 / background_tex.1.1 {
            let rect = Rect::new(
                0,
                (y * background_tex.1.1) as i32,
                background_tex.1.0,
                background_tex.1.1,
            );
            windowing.canvas.copy(background_tex.0, None, Some(rect))?;
        }
        Ok(())
    }

    fn render_command_hitboxes(&self, windowing: &mut Windowing) -> Result<(), String> {
        let comps = self
            .robot
            .commands
            .get_commands()
            .iter()
            .map(|(_, command)| {
                let shape = get_command_shape(command.get_data());
                shape
                    .iter()
                    .map(|p| {
                        p.iter()
                            .map(|s| point![s.x + command.pos.x, s.y + command.pos.y])
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        windowing.canvas.set_draw_color(Color::RGB(255, 0, 0));

        let mouse_state = windowing.event_pump.mouse_state();
        let (mouse_x, mouse_y) = (mouse_state.x(), mouse_state.y());

        comps.into_iter().try_for_each(|shape| {
            for w in shape.windows(2) {
                let a = (w[0].x, w[0].y);
                let b = (w[1].x, w[1].y);

                let (x1, y1) = self.to_screen(a.0, a.1, (mouse_x, mouse_y));
                let (x2, y2) = self.to_screen(b.0, b.1, (mouse_x, mouse_y));

                let a = Point::new(x1 as i32, y1 as i32);
                let b = Point::new(x2 as i32, y2 as i32);

                windowing.canvas.draw_line(a, b)?;
            }

            if shape.len() >= 2 {
                let a = (shape[0].x, shape[0].y);
                let b = (shape.last().unwrap().x, shape.last().unwrap().y);

                let (x1, y1) = self.to_screen(a.0, a.1, (mouse_x, mouse_y));
                let (x2, y2) = self.to_screen(b.0, b.1, (mouse_x, mouse_y));

                let a = Point::new(x1 as i32, y1 as i32);
                let b = Point::new(x2 as i32, y2 as i32);

                windowing.canvas.draw_line(a, b)?;
            }
            Ok::<(), String>(())
        })?;

        self.robot
            .commands
            .get_commands()
            .iter()
            .try_for_each(|(_, command)| {
                let pos = command.get_pos();
                let (x, y) = self.to_screen(pos.0, pos.1, (mouse_x, mouse_y));
                let rect = Rect::new(x - 1, y - 1, 3, 3);

                windowing.canvas.set_draw_color(Color::RGB(255, 0, 0));
                windowing.canvas.fill_rect(rect)?;

                let links = get_command_link_positions(command.get_data());
                links[0].iter().try_for_each(|p| {
                    let radius = (constants::PROGRAMMING_LINK_SIZE / 2.0
                        * constants::PIXELS_PER_METER
                        * self.zoom) as i32;
                    let (x, y) = self.to_screen(p.x + pos.0, p.y + pos.1, (mouse_x, mouse_y));
                    let rect =
                        Rect::new(x - radius, y - radius, radius as u32 * 2, radius as u32 * 2);

                    windowing.canvas.set_draw_color(Color::RGB(0, 255, 0));
                    windowing.canvas.draw_rect(rect)?;
                    Ok::<(), String>(())
                })?;

                links[1].iter().try_for_each(|p| {
                    let radius = (constants::PROGRAMMING_LINK_SIZE / 2.0
                        * constants::PIXELS_PER_METER
                        * self.zoom) as i32;
                    let (x, y) = self.to_screen(p.x + pos.0, p.y + pos.1, (mouse_x, mouse_y));
                    let rect =
                        Rect::new(x - radius, y - radius, radius as u32 * 2, radius as u32 * 2);

                    windowing.canvas.set_draw_color(Color::RGB(0, 0, 255));
                    windowing.canvas.draw_rect(rect)?;
                    Ok::<(), String>(())
                })?;
                Ok::<(), String>(())
            })?;

        Ok(())
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
                let a = (w[0].x, w[0].y);
                let b = (w[1].x, w[1].y);

                let (x1, y1) = self.to_screen(a.0, a.1, (mouse_x, mouse_y));
                let (x2, y2) = self.to_screen(b.0, b.1, (mouse_x, mouse_y));

                let a = Point::new(x1 as i32, y1 as i32);
                let b = Point::new(x2 as i32, y2 as i32);

                windowing.canvas.draw_line(a, b)?;
            }

            if shape.len() >= 2 {
                let a = (shape[0].x, shape[0].y);
                let b = (shape.last().unwrap().x, shape.last().unwrap().y);

                let (x1, y1) = self.to_screen(a.0, a.1, (mouse_x, mouse_y));
                let (x2, y2) = self.to_screen(b.0, b.1, (mouse_x, mouse_y));

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

        match self.state {
            BuildingState::Assembling => {
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

                if pressed && let Some(id) = self.selected_component_id {
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

                    self.selected_component_id = None;
                }
            }

            BuildingState::Programming => {
                if pressed && let Some(id) = self.selected_command_id {
                    if key == Keycode::DELETE {
                        self.robot.commands.remove(CommandHandle(id));
                        self.selected_command_id = None;
                    }

                    if self.editing_command_settings {
                        if let Some(command) = self.robot.get_command_mut(CommandHandle(id)) {
                            match command.get_data_mut() {
                                CommandData::OnKeyDown { key: key_mut }
                                | CommandData::OnKeyUp { key: key_mut } => {
                                    *key_mut = key;
                                    self.editing_command_settings = false;
                                }
                                CommandData::Const { val, val_str } => {
                                    if key == Keycode::Backspace && !val_str.is_empty() {
                                        val_str.pop();
                                        return;
                                    }
                                    if key == Keycode::Return {
                                        if let Ok(v) = val_str.parse::<f32>() {
                                            *val = v;
                                        }
                                        *val_str = val.to_string();

                                        self.editing_command_settings = false;

                                        return;
                                    }

                                    let character = key.name().chars().next().unwrap();
                                    if character.is_numeric() {
                                        *val_str += &format!("{}", character);
                                    } else if character == '.' && !val_str.contains('.') {
                                        *val_str += ".";
                                    } else if character == '-' && val_str.is_empty() {
                                        *val_str += "-";
                                    }
                                }
                                CommandData::SetState { comp, comp_str, .. } => {
                                    if key == Keycode::Backspace && !comp_str.is_empty() {
                                        comp_str.pop();
                                        return;
                                    }
                                    if key == Keycode::Return {
                                        if let Ok(v) = comp_str.parse::<u64>() {
                                            *comp = Some(ComponentHandle(v));
                                        } else {
                                            *comp = None;
                                        }

                                        *comp_str = if let Some(handle) = comp {
                                            handle.0.to_string()
                                        } else {
                                            "None".to_string()
                                        };

                                        self.editing_command_settings = false;

                                        return;
                                    }

                                    let character = key.name().chars().next().unwrap();
                                    if character.is_numeric() {
                                        *comp_str += &format!("{}", character);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
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

    fn handle_command_drag(
        &mut self,
        mouse_x: i32,
        mouse_y: i32,
        windowing: &Windowing,
        texture_handler: &TextureHandler,
    ) {
        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        if mouse_x > x {
            constants::COMMANDS_SET
                .iter()
                .enumerate()
                .for_each(|(i, data)| {
                    let y = i as i32 * box_tex.1.1 as i32 + self.scroll;

                    let rect = Rect::new(x, y, box_tex.1.0, box_tex.1.1);

                    if rect.contains_point(Point::new(mouse_x, mouse_y)) {
                        self.command_selected = Some(data.clone());
                    }
                });
        }
    }

    fn handle_component_select(&mut self, mouse_x: i32, mouse_y: i32) {
        let (x, y) = self.to_world(mouse_x, mouse_y);
        let mouse_pos = rapier2d::math::Point::new(x, y);

        self.selected_component_id = None;

        for (id, comp) in self.robot.components.iter() {
            let selected = comp
                .get_shapes()
                .into_iter()
                .any(|s| polygon::point_intersects_polygon(mouse_pos, &s));

            if selected {
                self.selected_component_id = Some(*id);
                break;
            }
        }
    }

    fn handle_command_select(&mut self, mouse_x: i32, mouse_y: i32) {
        if mouse_x < 300 {
            return;
        }

        let (x, y) = self.to_world(mouse_x, mouse_y);
        let mouse_pos = rapier2d::math::Point::new(x, y);

        self.selected_command_id = None;

        for (id, command) in self.robot.commands.get_commands().iter() {
            let selected = get_command_shape(command.get_data()).into_iter().any(|s| {
                let mut shape = s.to_vec();
                polygon::translate_polygon(&mut shape, command.get_pos().0, command.get_pos().1);
                polygon::point_intersects_polygon(mouse_pos, &shape)
            });

            if selected {
                self.selected_command_id = Some(*id);
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

        let state_rect = self.get_state_rect(texture_handler);
        if state_rect.contains_point(ml) {
            self.state = match self.state {
                BuildingState::Assembling => BuildingState::Programming,
                BuildingState::Programming => BuildingState::Assembling,
            }
        }

        let done_tex = texture_handler.get_texture(TextureId::DoneButton);

        let done_rect = Rect::new(
            state_rect.x(),
            state_rect.y() + state_rect.height() as i32 + 10,
            done_tex.1.0,
            done_tex.1.1,
        );

        if done_rect.contains_point(ml) {
            *game_data.state.write().await = State::InGame;
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

    fn handle_command_drop(
        &mut self,
        mouse_x: i32,
        mouse_y: i32,
        windowing: &Windowing,
        texture_handler: &TextureHandler<'_>,
    ) {
        let Some(data) = self.command_selected.take() else {
            return;
        };

        let (window_w, _window_h) = windowing.canvas.window().size();

        let box_tex = texture_handler.get_texture(TextureId::BuildingComponentBox);

        let x = window_w as i32 - box_tex.1.0 as i32;

        if mouse_x < x {
            let (x, y) = self.to_world(mouse_x, mouse_y);
            self.robot.add_command(Command::new(Vec2 { x, y }, data));
            self.command_selected = None;
        }
    }

    fn handle_command_link_start(&mut self, mouse_x: i32, mouse_y: i32) {
        let (world_x, world_y) = self.to_world(mouse_x, mouse_y);

        self.link_selected = None;

        'outer: for (id, command) in self.robot.commands.get_commands() {
            let outputs = constants::get_command_link_positions(command.get_data())[1];

            for i in 0..outputs.len() {
                let link = outputs[i];

                let x = link.x + command.get_pos().0 - constants::PROGRAMMING_LINK_SIZE / 2.0;
                let y = link.y + command.get_pos().1 - constants::PROGRAMMING_LINK_SIZE / 2.0;

                if world_x > x
                    && world_y > y
                    && world_x < x + constants::PROGRAMMING_LINK_SIZE
                    && world_y < y + constants::PROGRAMMING_LINK_SIZE
                {
                    self.link_selected = Some((CommandHandle(*id), i as u8));
                    break 'outer;
                }
            }
        }
    }

    fn handle_command_settings(&mut self, mouse_x: i32, mouse_y: i32, mouse_btn: MouseButton) {
        if let Some(command_id) = self.selected_command_id {
            if let Some(command) = self.robot.get_command_mut(CommandHandle(command_id)) {
                self.editing_command_settings = false;

                if mouse_x < 300 && mouse_y < constants::FONT_HEIGHT {
                    self.editing_command_settings = true;
                }

                if let CommandData::SetState { state, .. } = command.get_data_mut() {
                    if mouse_x < 300 && mouse_y > 76 && mouse_y < 56 + 76 {
                        if mouse_btn == MouseButton::Left {
                            *state = state.next();
                        } else if mouse_btn == MouseButton::Right {
                            *state = state.prev();
                        }
                    }
                }
            }
        }
    }

    fn handle_command_link_end(&mut self, mouse_x: i32, mouse_y: i32) {
        let Some(link_selected) = self.link_selected else {
            return;
        };

        let (world_x, world_y) = self.to_world(mouse_x, mouse_y);
        let mut input_link: Option<(CommandHandle, u8)> = None;

        'outer: for (id, command) in self.robot.commands.get_commands_mut() {
            let inputs = constants::get_command_link_positions(command.get_data())[0];

            for i in 0..inputs.len() {
                let link = inputs[i];

                let x = link.x + command.get_pos().0 - constants::PROGRAMMING_LINK_SIZE / 2.0;
                let y = link.y + command.get_pos().1 - constants::PROGRAMMING_LINK_SIZE / 2.0;

                if world_x > x
                    && world_y > y
                    && world_x < x + constants::PROGRAMMING_LINK_SIZE
                    && world_y < y + constants::PROGRAMMING_LINK_SIZE
                {
                    input_link = Some((CommandHandle(*id), i as u8));
                    break 'outer;
                }
            }
        }

        if let Some(link) = input_link {
            if let Some(command) = self.robot.get_command_mut(link_selected.0) {
                command.connect(link.0, link_selected.1, link.1);
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

    pub(in crate::client) fn to_screen(
        &self,
        mut x: f32,
        mut y: f32,
        mp: (i32, i32),
    ) -> (i32, i32) {
        let (mouse_x, mouse_y) = mp;

        if self.pan_button_down {
            let (mouse_world_x, mouse_world_y) = self.to_world(mouse_x, mouse_y);

            x += mouse_world_x - self.pan_button_down_x;
            y += mouse_world_y - self.pan_button_down_y;
        }

        (
            ((x - self.dx) * self.zoom * constants::PIXELS_PER_METER) as i32,
            ((y - self.dy) * self.zoom * constants::PIXELS_PER_METER) as i32,
        )
    }

    pub fn get_robot(&self) -> &Robot {
        &self.robot
    }

    fn get_state_rect(&self, texture_handler: &TextureHandler) -> Rect {
        let size = texture_handler
            .get_texture(TextureId::BuildingStateButton)
            .1;

        let x = if matches!(self.state, BuildingState::Assembling)
            && self.selected_component_id.is_some()
            || matches!(self.state, BuildingState::Programming)
                && self.selected_command_id.is_some()
        {
            310
        } else {
            10
        };

        Rect::new(x, 10, size.0, size.1 / 2)
    }
}
