use std::f32::consts::PI;

use rapier2d::parry::utils::hashmap::HashMap;
use sdl2::{
    keyboard::Keycode,
    rect::{Point, Rect},
    render::Canvas,
    video::Window,
};

use crate::{
    client::building::BuildingMenu,
    constants::{get_command_texture, get_selected_command_texture},
    game::{
        component::{ComponentActivationState, ComponentHandle},
        world::{KeyState, World},
    },
    polygon::Vec2,
    texture_handler::TextureHandler,
};

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy)]
pub enum Data {
    Number(f32),   // #df7126 // orange
    Boolean(bool), // #5b6ee1 // blue
    Action(bool),  // #ac3232 // red
    None,
}

#[derive(Clone, Debug)]
pub struct Command {
    pub pos: Vec2,
    inputs: Vec<Option<(CommandHandle, u8)>>,
    outputs: Vec<Option<Vec<(CommandHandle, u8)>>>,
    data: CommandData,
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

    pub fn remove(&mut self, handle: CommandHandle) -> Option<Command> {
        self.commands.remove(&handle.0)
    }

    pub fn get_commands(&self) -> &HashMap<u64, Command> {
        &self.commands
    }
}

impl Command {
    fn get_inputs_outputs(kind: &CommandData) -> (usize, usize) {
        match kind {
            CommandData::OnKeyDown { .. } => (0, 1),
            CommandData::OnKeyUp { .. } => (0, 1),
            CommandData::SetState { .. } => (1, 1),
            CommandData::Add => (2, 1),
            CommandData::Sub => (2, 1),
            CommandData::Neg => (1, 1),
            CommandData::Mul => (2, 1),
            CommandData::Div => (2, 1),
            CommandData::Sqrt => (1, 1),
            CommandData::Pow => (2, 1),
            CommandData::Sin => (1, 1),
            CommandData::Cos => (1, 1),
            CommandData::Tan => (1, 1),
            CommandData::Asin => (1, 1),
            CommandData::Acos => (1, 1),
            CommandData::Atan => (1, 1),
            CommandData::Atan2 => (2, 1),
            CommandData::LessThan => (2, 1),
            CommandData::GreaterThan => (2, 1),
            CommandData::Const { .. } => (0, 1),
            CommandData::True => (0, 1),
            CommandData::False => (0, 1),
            CommandData::And => (2, 1),
            CommandData::Or => (2, 1),
            CommandData::Xor => (2, 1),
            CommandData::Not => (1, 1),
            CommandData::Ternary => (3, 1),
            CommandData::If => (1, 2),
        }
    }

    pub fn new(pos: Vec2, kind: CommandData) -> Self {
        let (inputs, outputs) = Self::get_inputs_outputs(&kind);

        Command {
            pos,
            inputs: vec![None; inputs],
            outputs: vec![None; outputs],
            data: kind,
        }
    }

    pub fn tick(&mut self, world: &World) {}

    pub fn evaluate(&self, world: &mut World, command_set: &mut CommandSet) -> Data {
        let inputs = self
            .inputs
            .iter()
            .map(|el| {
                let Some((handle, id)) = el else {
                    return Data::None;
                };

                let Some(command) = command_set.get(*handle) else {
                    return Data::None;
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
                Data::None
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
                let Some(handles) = (if bools[0] {
                    &self.outputs[0]
                } else {
                    &self.outputs[1]
                }) else {
                    return Data::None;
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

                Data::None
            }
        }
    }

    pub(in crate::client) fn get_data(&self) -> &CommandData {
        &self.data
    }

    pub(in crate::client) fn get_pos(&self) -> (f32, f32) {
        (self.pos.x, self.pos.y)
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
