use std::f32::consts::PI;

use rapier2d::parry::utils::hashmap::HashMap;
use sdl2::{
    keyboard::Keycode,
    pixels::Color,
    rect::{Point, Rect},
    render::Canvas,
    video::Window,
};

use crate::{
    client::building::BuildingMenu,
    constants::{self, get_command_texture, get_selected_command_texture},
    game::{
        component::{ComponentActivationState, ComponentHandle},
        world::{KeyState, World},
    },
    polygon::Vec2,
    texture_handler::TextureHandler,
    windowing::Windowing,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommandHandle(pub u64);

#[derive(Clone, Debug)]
pub enum CommandData {
    OnKeyDown {
        key: Keycode,
    },
    OnKeyUp {
        key: Keycode,
    },

    SetState {
        comps: Vec<ComponentHandle>,
        state: ComponentActivationState,
    },

    Const {
        val: f32,
    },

    True,
    False,

    Add,
    Sub,
    Neg,
    Mul,
    Div,
    Sqrt,
    Pow,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,

    LessThan,
    GreaterThan,

    And,
    Or,
    Xor,
    Not,

    Ternary,
    If,
}

#[derive(Debug, Clone, Copy)]
pub enum Data {
    Number(f32),   // #df7126 // orange
    Boolean(bool), // #5b6ee1 // blue
    Action(bool),  // #ac3232 // red
}

#[derive(Default, Clone)]
pub struct CommandSet {
    commands: HashMap<u64, Command>,
    next_id: u64,
}

impl CommandSet {
    pub fn new() -> CommandSet {
        CommandSet {
            commands: HashMap::default(),
            next_id: 0,
        }
    }

    pub fn add_command(&mut self, command: Command) -> CommandHandle {
        let id = self.next_id;
        self.next_id += 1;

        self.commands.insert(id, command);

        CommandHandle(id)
    }

    pub fn get(&self, handle: CommandHandle) -> Option<&Command> {
        self.commands.get(&handle.0)
    }

    pub fn get_mut(&mut self, handle: CommandHandle) -> Option<&mut Command> {
        self.commands.get_mut(&handle.0)
    }

    pub fn remove(&mut self, to_remove: CommandHandle) -> Option<Command> {
        let mut outputs_to_remove = vec![];

        self.commands.iter_mut().for_each(|(command_id, command)| {
            command
                .outputs
                .iter()
                .enumerate()
                .for_each(|(output_id, c)| {
                    c.iter().for_each(|(handle, input_id)| {
                        if *handle == to_remove {
                            outputs_to_remove.push((
                                CommandHandle(*command_id),
                                output_id as u64,
                                *input_id as u64,
                            ));
                        }
                    })
                });
        });

        outputs_to_remove.iter().for_each(|(handle, o, i)| {
            if let Some(command) = self.commands.get_mut(&handle.0) {
                command.disconnect(*o, *i);
            }
        });

        self.commands.remove(&to_remove.0)
    }

    pub fn get_commands(&self) -> &HashMap<u64, Command> {
        &self.commands
    }

    pub fn get_commands_mut(&mut self) -> &mut HashMap<u64, Command> {
        &mut self.commands
    }
}

#[derive(Clone, Debug)]
pub struct Command {
    pub pos: Vec2,
    inputs: Vec<Option<(CommandHandle, u8)>>,
    outputs: Vec<Vec<(CommandHandle, u8)>>,
    data: CommandData,
}

impl Command {
    pub fn new(pos: Vec2, kind: CommandData) -> Self {
        let (inputs, outputs) = constants::get_command_io_counts(&kind);

        Command {
            pos,
            inputs: vec![None; inputs],
            outputs: vec![vec![]; outputs],
            data: kind,
        }
    }

    pub fn tick(&mut self, world: &World) {}

    pub(in crate::client) fn render(
        &self,
        canvas: &mut Canvas<Window>,
        texture_handler: &TextureHandler,
        scale: f32,
        building_menu: &BuildingMenu,
        selected: bool,
        ml: (i32, i32),
    ) -> Result<(), String> {
        let mut tex = if selected {
            texture_handler.get_texture(get_selected_command_texture(&self.data))
        } else {
            texture_handler.get_texture(get_command_texture(&self.data))
        };

        tex.1.0 = (tex.1.0 as f32 * scale) as u32;
        tex.1.1 = (tex.1.1 as f32 * scale) as u32;

        let (x, y) = building_menu.to_screen(self.pos.x, self.pos.y, ml);

        let dst = Rect::new(
            (x as f32 - tex.1.0 as f32 / 2.0) as i32,
            (y as f32 - tex.1.1 as f32 / 2.0) as i32,
            tex.1.0,
            tex.1.1,
        );

        canvas.copy(tex.0, None, Some(dst))?;

        self.outputs.iter().enumerate().try_for_each(|(o_id, o)| {
            o.iter().try_for_each(|(handle, i_id)| {
                let command = building_menu.get_robot().get_command(*handle);
                if let Some(command) = command {
                    let data_kind =
                        constants::get_command_io_type(command.get_data()).0[*i_id as usize];

                    canvas.set_draw_color(match data_kind {
                        Data::Number(_) => Color::RGB(0xdf, 0x71, 0x26),
                        Data::Boolean(_) => Color::RGB(0x5b, 0x63, 0xe1),
                        Data::Action(_) => Color::RGB(0xac, 0x32, 0x32),
                    });

                    let (o_x, o_y) = {
                        let output_rel_pos =
                            constants::get_command_link_positions(self.get_data())[1][o_id];

                        building_menu.to_screen(
                            output_rel_pos.x + self.get_pos().0
                                - constants::PROGRAMMING_LINK_SIZE / 2.0,
                            output_rel_pos.y + self.get_pos().1
                                - constants::PROGRAMMING_LINK_SIZE / 2.0,
                            ml,
                        )
                    };

                    let (i_x, i_y) = {
                        let input_rel_pos =
                            constants::get_command_link_positions(command.get_data())[0]
                                [*i_id as usize];

                        building_menu.to_screen(
                            input_rel_pos.x + command.get_pos().0
                                - constants::PROGRAMMING_LINK_SIZE / 2.0,
                            input_rel_pos.y + command.get_pos().1
                                - constants::PROGRAMMING_LINK_SIZE / 2.0,
                            ml,
                        )
                    };

                    let rect_a = Rect::new(
                        o_x.min((o_x + i_x) / 2),
                        o_y,
                        ((i_x - o_x) / 2).abs() as u32
                            + ((3 * constants::TEXTURE_SCALE) as f32 * scale) as u32,
                        ((3 * constants::TEXTURE_SCALE) as f32 * scale) as u32,
                    );

                    let rect_b = Rect::new(
                        (i_x + o_x) / 2,
                        i_y.min(o_y),
                        ((3 * constants::TEXTURE_SCALE) as f32 * scale) as u32,
                        (o_y - i_y).abs() as u32
                            + ((3 * constants::TEXTURE_SCALE) as f32 * scale) as u32,
                    );

                    let rect_c = Rect::new(
                        i_x.min((o_x + i_x) / 2),
                        i_y,
                        ((i_x - o_x) / 2).abs() as u32,
                        ((3 * constants::TEXTURE_SCALE) as f32 * scale) as u32,
                    );

                    canvas.fill_rect(rect_a)?;
                    canvas.fill_rect(rect_b)?;
                    canvas.fill_rect(rect_c)?;
                }
                Ok::<(), String>(())
            })?;
            Ok::<(), String>(())
        })?;

        Ok(())
    }

    pub fn connect(&mut self, handle: CommandHandle, o_id: u8, i_id: u8) {
        self.outputs[o_id as usize].push((handle, i_id));
    }

    pub fn disconnect(&mut self, o: u64, i: u64) {
        self.outputs[o as usize].remove(i as usize);
    }

    pub fn evaluate(&self, world: &mut World, command_set: &mut CommandSet) -> Data {
        let inputs = self
            .inputs
            .iter()
            .map(|el| {
                let Some((handle, id)) = el else {
                    return Data::Number(0.0);
                };

                let Some(command) = command_set.get(*handle) else {
                    return Data::Number(0.0);
                };

                command.clone().evaluate(world, command_set)
            })
            .collect::<Vec<_>>();

        let nums = inputs
            .iter()
            .map(|i| match i {
                Data::Number(x) => *x,
                _ => 0.0,
            })
            .collect::<Vec<_>>();

        let bools = inputs
            .iter()
            .map(|i| match i {
                Data::Boolean(x) => *x,
                _ => false,
            })
            .collect::<Vec<_>>();

        match &self.data {
            CommandData::OnKeyDown { key } => {
                let Some(state) = world.keys.get(key) else {
                    return Data::Boolean(false);
                };
                Data::Boolean(matches!(state, KeyState::JustPressed))
            }
            CommandData::OnKeyUp { key } => {
                let Some(state) = world.keys.get(key) else {
                    return Data::Boolean(false);
                };
                Data::Boolean(matches!(state, KeyState::JustReleased))
            }
            CommandData::SetState { comps, state } => {
                for h in comps {
                    if let Some(comp) = world.components.get_mut(*h) {
                        comp.set_state(*state);
                    }
                }
                Data::Number(0.0)
            }
            CommandData::Add => Data::Number(nums[0] + nums[1]),
            CommandData::Sub => Data::Number(nums[0] - nums[1]),
            CommandData::Neg => Data::Number(-nums[0]),
            CommandData::Mul => Data::Number(nums[0] * nums[1]),
            CommandData::Div => Data::Number(nums[0] / nums[1]),
            CommandData::Sqrt => Data::Number(nums[0].sqrt()),
            CommandData::Pow => Data::Number(nums[0].powf(nums[1])),
            CommandData::Sin => Data::Number(nums[0].sin()),
            CommandData::Cos => Data::Number(nums[0].cos()),
            CommandData::Tan => Data::Number(nums[0].tan()),
            CommandData::Asin => Data::Number(nums[0].asin()),
            CommandData::Acos => Data::Number(nums[0].acos()),
            CommandData::Atan => Data::Number(nums[0].atan()),
            CommandData::Atan2 => Data::Number(nums[0].atan2(nums[1])),
            CommandData::GreaterThan => Data::Boolean(nums[0] > nums[1]),
            CommandData::LessThan => Data::Boolean(nums[0] < nums[1]),
            CommandData::Const { val } => Data::Number(*val),
            CommandData::True => Data::Boolean(true),
            CommandData::False => Data::Boolean(false),
            CommandData::And => Data::Boolean(bools[0] && bools[1]),
            CommandData::Or => Data::Boolean(bools[0] || bools[1]),
            CommandData::Xor => Data::Boolean(bools[0] ^ bools[1]),
            CommandData::Not => Data::Boolean(!bools[0]),
            CommandData::Ternary => {
                if bools[0] {
                    inputs[1]
                } else {
                    inputs[2]
                }
            }
            CommandData::If => {
                let handles = if bools[0] {
                    &self.outputs[0]
                } else {
                    &self.outputs[1]
                };

                handles
                    .iter()
                    .map(|(id, _)| command_set.get(*id))
                    .flatten()
                    .map(|c| c.clone())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .for_each(|c| {
                        c.evaluate(world, command_set);
                    });

                Data::Number(0.0)
            }
        }
    }

    pub(in crate::client) fn move_to(&mut self, x: f32, y: f32) {
        self.pos = Vec2 { x, y };
    }

    pub(in crate::client) fn get_data(&self) -> &CommandData {
        &self.data
    }

    pub(in crate::client) fn get_pos(&self) -> (f32, f32) {
        (self.pos.x, self.pos.y)
    }

    pub(in crate::client) fn get_outputs(&self) -> &Vec<Vec<(CommandHandle, u8)>> {
        &self.outputs
    }
}

pub(crate) fn render_command(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    command_data: &CommandData,
    x: i32,
    y: i32,
    rot: f32,
    scale: f32,
    selected: bool,
) -> Result<(), String> {
    let mut tex = if selected {
        texture_handler.get_texture(get_selected_command_texture(command_data))
    } else {
        texture_handler.get_texture(get_command_texture(command_data))
    };

    tex.1.0 = (tex.1.0 as f32 * scale) as u32;
    tex.1.1 = (tex.1.1 as f32 * scale) as u32;

    let dst = Rect::new(
        (x as f32 - tex.1.0 as f32 / 2.0) as i32,
        (y as f32 - tex.1.1 as f32 / 2.0) as i32,
        tex.1.0,
        tex.1.1,
    );
    let center = Point::new(dst.width() as i32 / 2, dst.height() as i32 / 2);

    canvas.copy_ex(
        tex.0,
        None,
        Some(dst),
        (rot * 180.0 / PI) as f64,
        center,
        false,
        false,
    )?;

    Ok(())
}
