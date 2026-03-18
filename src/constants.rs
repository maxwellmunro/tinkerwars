use crate::client::programming::CommandData;
use crate::game::component::{ComponentActivationState, ComponentKind};
use crate::texture_handler::TextureId;
use rapier2d::math::Point;
use sdl2::keyboard::Keycode;

pub const SERVER_CONNECT_TIMEOUT_MS: u64 = 5000;
pub const PHYSICS_STEPS: usize = 10;
pub const CHAT_MSG_LIMIT: usize = 1000;

pub const MOTOR_MAX_TORQUE: f32 = 10000.0;
pub const PISTON_MAX_FORCE: f32 = 10000.0;
pub const PISTON_LIMITS: [f32; 2] = [0.0, 6.0];
pub const TEXTURE_SCALE: u32 = 4;
pub const PIXELS_PER_METER: f32 = 20.0;

pub const PART_LIST_MOVE_FAC: f32 = 500.0;
pub const PART_MIN_STICK_DIST: f32 = 20.0;

pub const MAX_FRAME_TIME: f32 = 0.1;

const PART_SELECT_MAX_DIST: i32 = 50;
pub const PART_SELECT_SQUARE_DIST: i32 = PART_SELECT_MAX_DIST.pow(2);
pub const PART_SELECT_MS_PER_UNIT: u32 = 5000;
pub const PART_SELECT_POS_ATTEMPTS: u32 = 10;

pub mod window {
    pub const SERVER_TITLE: &str = "Combat Protocol - Server";
    pub const CLIENT_TITLE: &str = "Combat Protocol";
    pub const DEFAULT_WIDTH: u32 = 800;
    pub const DEFAULT_HEIGHT: u32 = 600;
}

pub mod networking {
    pub const SERVER_PORT: u32 = 8000;
    pub const BUFFER_SIZE: usize = 4096;
    pub const CHANNEL_SIZE: usize = 128;
    pub const PLAYER_UPDATE_PERIOD: u64 = 50;
}

pub mod component_shapes {
    use rapier2d::math::Point;
    use rapier2d::na::point;
    use rapier2d::prelude::nalgebra;

    pub const ARM_LARGE: &[&[Point<f32>]] = &[&[
        point![-6.0, -0.8],
        point![6.0, -0.8],
        point![6.4, -0.4],
        point![6.4, 0.4],
        point![6.0, 0.8],
        point![-6.0, 0.8],
        point![-6.4, 0.4],
        point![-6.4, -0.4],
    ]];

    pub const ARM_MEDIUM: &[&[Point<f32>]] = &[&[
        point![-2.8, -0.8],
        point![2.8, -0.8],
        point![3.2, -0.4],
        point![3.2, 0.4],
        point![2.8, 0.8],
        point![-2.8, 0.8],
        point![-3.2, 0.4],
        point![-3.2, -0.4],
    ]];

    pub const ARM_SMALL: &[&[Point<f32>]] = &[&[
        point![-1.2, -0.8],
        point![1.2, -0.8],
        point![1.6, -0.4],
        point![1.6, 0.4],
        point![1.2, 0.8],
        point![-1.2, 0.8],
        point![-1.6, 0.4],
        point![-1.6, -0.4],
    ]];

    pub const ARM_TINY: &[&[Point<f32>]] = &[&[
        point![-0.6, -0.4],
        point![0.6, -0.4],
        point![0.8, -0.2],
        point![0.8, 0.2],
        point![0.6, 0.4],
        point![-0.6, 0.4],
        point![-0.8, 0.2],
        point![-0.8, -0.2],
    ]];

    pub const BODY: &[&[Point<f32>]] = &[&[
        point![-2.4, -3.2],
        point![2.4, -3.2],
        point![3.2, -2.4],
        point![3.2, 2.4],
        point![2.4, 3.2],
        point![-2.4, 3.2],
        point![-3.2, 2.4],
        point![-3.2, -2.4],
    ]];

    pub const MOTOR: &[&[Point<f32>]] = &[
        &[
            point![1.6, 0.0],
            point![1.4782, 0.6123],
            point![1.131, 1.131],
            point![0.6123, 1.4782],
            point![0.0, 1.6],
            point![-0.6123, 1.4782],
            point![-1.131, 1.131],
            point![-1.4782, 0.6123],
            point![-1.6, 0.0],
            point![-1.4782, -0.6123],
            point![-1.131, -1.131],
            point![-0.6123, -1.4782],
            point![0.0, -1.6],
            point![0.6123, -1.4782],
            point![1.131, -1.131],
            point![1.4782, -0.6123],
        ],
        &[
            point![1.6, 0.0],
            point![1.4782, 0.6123],
            point![1.131, 1.131],
            point![0.6123, 1.4782],
            point![0.0, 1.6],
            point![-0.6123, 1.4782],
            point![-1.131, 1.131],
            point![-1.4782, 0.6123],
            point![-1.6, 0.0],
            point![-1.4782, -0.6123],
            point![-1.131, -1.131],
            point![-0.6123, -1.4782],
            point![0.0, -1.6],
            point![0.6123, -1.4782],
            point![1.131, -1.131],
            point![1.4782, -0.6123],
        ],
    ];

    pub const PISTON: &[&[Point<f32>]] = &[
        &[
            point![-3.2, -0.4],
            point![-2.8, -0.8],
            point![2.8, -0.8],
            point![3.2, -0.4],
            point![3.2, 0.4],
            point![2.8, 0.8],
            point![-2.8, 0.8],
            point![-3.2, 0.4],
        ],
        &[
            point![-3.2, -0.4],
            point![-2.6, -0.6],
            point![2.6, -0.6],
            point![3.2, -0.4],
            point![3.2, 0.4],
            point![2.6, 0.6],
            point![-2.6, 0.6],
            point![-3.2, 0.4],
        ],
    ];

    pub const SCREWS: &[&[Point<f32>]] = &[&[
        point![-1.0, -1.0],
        point![1.0, -1.0],
        point![1.0, 1.0],
        point![-1.0, 1.0],
    ]];
}

pub const fn get_component_shape(kind: ComponentKind) -> &'static [&'static [Point<f32>]] {
    match kind {
        ComponentKind::ArmLarge => component_shapes::ARM_LARGE,
        ComponentKind::ArmMedium => component_shapes::ARM_MEDIUM,
        ComponentKind::ArmSmall => component_shapes::ARM_SMALL,
        ComponentKind::ArmTiny => component_shapes::ARM_TINY,
        ComponentKind::Body => component_shapes::BODY,
        ComponentKind::Motor => component_shapes::MOTOR,
        ComponentKind::Piston => component_shapes::PISTON,
        ComponentKind::Screw => component_shapes::SCREWS,
    }
}

pub const fn get_component_health(kind: ComponentKind) -> f32 {
    match kind {
        ComponentKind::ArmLarge => 200.0,
        ComponentKind::ArmMedium => 100.0,
        ComponentKind::ArmSmall => 50.0,
        ComponentKind::ArmTiny => 25.0,
        ComponentKind::Body => 500.0,
        ComponentKind::Motor => 200.0,
        ComponentKind::Piston => 200.0,
        ComponentKind::Screw => 0.0,
    }
}

pub const fn get_component_texture(kind: ComponentKind) -> TextureId {
    match kind {
        ComponentKind::ArmLarge => TextureId::ComponentArmLarge,
        ComponentKind::ArmMedium => TextureId::ComponentArmMedium,
        ComponentKind::ArmSmall => TextureId::ComponentArmSmall,
        ComponentKind::ArmTiny => TextureId::ComponentArmTiny,
        ComponentKind::Body => TextureId::ComponentBody,
        ComponentKind::Motor => TextureId::ComponentMotor,
        ComponentKind::Piston => TextureId::ComponentPiston,
        ComponentKind::Screw => TextureId::ComponentScrew,
    }
}

pub const fn get_component_icon_texture(kind: ComponentKind) -> TextureId {
    match kind {
        ComponentKind::ArmLarge => TextureId::IconArmLarge,
        ComponentKind::ArmMedium => TextureId::IconArmMedium,
        ComponentKind::ArmSmall => TextureId::IconArmSmall,
        ComponentKind::ArmTiny => TextureId::IconArmTiny,
        ComponentKind::Body => TextureId::IconBody,
        ComponentKind::Motor => TextureId::IconMotor,
        ComponentKind::Piston => TextureId::IconPiston,
        ComponentKind::Screw => TextureId::IconScrew,
    }
}

pub const fn get_mask_texture(kind: ComponentKind) -> TextureId {
    match kind {
        ComponentKind::ArmLarge => TextureId::MaskArmLarge,
        ComponentKind::ArmMedium => TextureId::MaskArmMedium,
        ComponentKind::ArmSmall => TextureId::MaskArmSmall,
        ComponentKind::ArmTiny => TextureId::MaskArmTiny,
        ComponentKind::Body => TextureId::MaskBody,
        ComponentKind::Motor => TextureId::MaskMotor,
        ComponentKind::Piston => TextureId::MaskPiston,
        ComponentKind::Screw => TextureId::MaskScrew,
    }
}

pub mod command_shapes {
    use rapier2d::math::Point;
    use rapier2d::na::point;
    use rapier2d::prelude::nalgebra;

    pub const ACOS: &[&[Point<f32>]] = &[&[
        point![-3.5, -2.3],
        point![3.5, -2.3],
        point![3.5, 2.3],
        point![-3.5, 2.3],
    ]];

    pub const ADD: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const AND: &[&[Point<f32>]] = &[&[
        point![-5.0, -2.3],
        point![3.3, -2.3],
        point![5.0, -0.75],
        point![5.0, 0.75],
        point![3.4, 2.3],
        point![-5.0, 2.3],
    ]];

    pub const ASIN: &[&[Point<f32>]] = &[&[
        point![-3.5, -2.3],
        point![3.5, -2.3],
        point![3.5, 2.3],
        point![-3.5, 2.3],
    ]];

    pub const ATAN: &[&[Point<f32>]] = &[&[
        point![-3.5, -2.3],
        point![3.5, -2.3],
        point![3.5, 2.3],
        point![-3.5, 2.3],
    ]];

    pub const ATAN2: &[&[Point<f32>]] = &[&[
        point![-4.2, -2.3],
        point![4.2, -2.3],
        point![4.2, 2.3],
        point![-4.2, 2.3],
    ]];

    pub const CONST: &[&[Point<f32>]] = &[&[
        point![-2.6, -2.3],
        point![2.0, -2.3],
        point![2.0, 2.3],
        point![-2.6, 2.3],
    ]];

    pub const COS: &[&[Point<f32>]] = &[&[
        point![-2.8, -2.3],
        point![2.8, -2.3],
        point![2.8, 2.3],
        point![-2.8, 2.3],
    ]];

    pub const DIV: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const FALSE: &[&[Point<f32>]] = &[&[
        point![-2.6, -2.3],
        point![2.0, -2.3],
        point![2.0, 2.3],
        point![-2.6, 2.3],
    ]];

    pub const GREATER_THAN: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
        
    ]];

    pub const IF: &[&[Point<f32>]] =
        &[&[point![4.75, -2.25], point![-4.25, 0.0], point![4.75, 2.25]]];

    pub const LESS_THAN: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const MUL: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const NEG: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const NOT: &[&[Point<f32>]] =
        &[&[point![-4.75, -2.25], point![4.25, 0.0], point![-4.75, 2.25]]];

    pub const ONKEYDOWN: &[&[Point<f32>]] = &[&[
        point![-7.3, -2.3],
        point![6.70, -2.3],
        point![6.7, 2.3],
        point![-7.3, 2.3],
    ]];

    pub const ONKEYUP: &[&[Point<f32>]] = &[&[
        point![-5.9, -2.3],
        point![5.3, -2.3],
        point![5.3, 2.3],
        point![-5.9, 2.3],
    ]];

    pub const OR: &[&[Point<f32>]] = &[&[
        point![-5.6, -2.3],
        point![3.4, -2.3],
        point![5.0, -0.75],
        point![5.0, 0.75],
        point![3.4, 2.3],
        point![-5.4, 2.3],
        point![-4.4, 0.0],
    ]];

    pub const POW: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const SETSTATE: &[&[Point<f32>]] = &[&[
        point![-6.0, -2.3],
        point![6.6, -2.3],
        point![6.6, 2.3],
        point![-6.0, 2.3],
    ]];

    pub const SIN: &[&[Point<f32>]] = &[&[
        point![-2.8, -2.3],
        point![2.8, -2.3],
        point![2.8, 2.3],
        point![-2.8, 2.3],
    ]];

    pub const SQRT: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const SUB: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const TAN: &[&[Point<f32>]] = &[&[
        point![-2.8, -2.3],
        point![2.8, -2.3],
        point![2.8, 2.3],
        point![-2.8, 2.3],
    ]];

    pub const TERNARY: &[&[Point<f32>]] = &[&[
        point![-2.3, -2.3],
        point![2.3, -2.3],
        point![2.3, 2.3],
        point![-2.3, 2.3],
    ]];

    pub const TRUE: &[&[Point<f32>]] = &[&[
        point![-2.6, -2.3],
        point![2.0, -2.3],
        point![2.0, 2.3],
        point![-2.6, 2.3],
    ]];

    pub const XOR: &[&[Point<f32>]] = &[&[
        point![-5.6, -2.3],
        point![3.4, -2.3],
        point![5.0, -0.75],
        point![5.0, 0.75],
        point![3.4, 2.3],
        point![-5.4, 2.3],
        point![-4.4, 0.0],
    ]];
}

pub const COMMANDS_SET: &[CommandData] = &[
    CommandData::OnKeyDown { key: Keycode::A },
    CommandData::OnKeyUp { key: Keycode::A },
    CommandData::SetState {
        comps: vec![],
        state: ComponentActivationState::None,
    },
    CommandData::Const { val: 0.0 },
    CommandData::True,
    CommandData::False,
    CommandData::Add,
    CommandData::Sub,
    CommandData::Neg,
    CommandData::Mul,
    CommandData::Div,
    CommandData::Sqrt,
    CommandData::Pow,
    CommandData::Sin,
    CommandData::Cos,
    CommandData::Tan,
    CommandData::Asin,
    CommandData::Acos,
    CommandData::Atan,
    CommandData::Atan2,
    CommandData::LessThan,
    CommandData::GreaterThan,
    CommandData::And,
    CommandData::Or,
    CommandData::Xor,
    CommandData::Not,
    CommandData::Ternary,
    CommandData::If,
];

pub const fn get_command_shape(data: &CommandData) -> &'static [&'static [Point<f32>]] {
    match data {
        CommandData::OnKeyDown { .. } => command_shapes::ONKEYDOWN,
        CommandData::OnKeyUp { .. } => command_shapes::ONKEYUP,
        CommandData::SetState { .. } => command_shapes::SETSTATE,
        CommandData::Const { .. } => command_shapes::CONST,
        CommandData::True => command_shapes::TRUE,
        CommandData::False => command_shapes::FALSE,
        CommandData::Add => command_shapes::ADD,
        CommandData::Sub => command_shapes::SUB,
        CommandData::Neg => command_shapes::NEG,
        CommandData::Mul => command_shapes::MUL,
        CommandData::Div => command_shapes::DIV,
        CommandData::Sqrt => command_shapes::SQRT,
        CommandData::Pow => command_shapes::POW,
        CommandData::Sin => command_shapes::SIN,
        CommandData::Cos => command_shapes::COS,
        CommandData::Tan => command_shapes::TAN,
        CommandData::Asin => command_shapes::ASIN,
        CommandData::Acos => command_shapes::ACOS,
        CommandData::Atan => command_shapes::ATAN,
        CommandData::Atan2 => command_shapes::ATAN2,
        CommandData::LessThan => command_shapes::LESS_THAN,
        CommandData::GreaterThan => command_shapes::GREATER_THAN,
        CommandData::And => command_shapes::AND,
        CommandData::Or => command_shapes::OR,
        CommandData::Xor => command_shapes::XOR,
        CommandData::Not => command_shapes::NOT,
        CommandData::Ternary => command_shapes::TERNARY,
        CommandData::If => command_shapes::IF,
    }
}

pub const fn get_command_texture(data: &CommandData) -> TextureId {
    match data {
        CommandData::OnKeyDown { .. } => TextureId::ProgrammingOnKeyDown,
        CommandData::OnKeyUp { .. } => TextureId::ProgrammingOnKeyUp,
        CommandData::SetState { .. } => TextureId::ProgrammingSetState,
        CommandData::Const { .. } => TextureId::ProgrammingConst,
        CommandData::True => TextureId::ProgrammingTrue,
        CommandData::False => TextureId::ProgrammingFalse,
        CommandData::Add => TextureId::ProgrammingAdd,
        CommandData::Sub => TextureId::ProgrammingSub,
        CommandData::Neg => TextureId::ProgrammingNeg,
        CommandData::Mul => TextureId::ProgrammingMul,
        CommandData::Div => TextureId::ProgrammingDiv,
        CommandData::Sqrt => TextureId::ProgrammingSqrt,
        CommandData::Pow => TextureId::ProgrammingPow,
        CommandData::Sin => TextureId::ProgrammingSin,
        CommandData::Cos => TextureId::ProgrammingCos,
        CommandData::Tan => TextureId::ProgrammingTan,
        CommandData::Asin => TextureId::ProgrammingAsin,
        CommandData::Acos => TextureId::ProgrammingAcos,
        CommandData::Atan => TextureId::ProgrammingAtan,
        CommandData::Atan2 => TextureId::ProgrammingAtan2,
        CommandData::LessThan => TextureId::ProgrammingLessThan,
        CommandData::GreaterThan => TextureId::ProgrammingGreaterThan,
        CommandData::And => TextureId::ProgrammingAnd,
        CommandData::Or => TextureId::ProgrammingOr,
        CommandData::Xor => TextureId::ProgrammingXor,
        CommandData::Not => TextureId::ProgrammingNot,
        CommandData::Ternary => TextureId::ProgrammingTernary,
        CommandData::If => TextureId::ProgrammingIf,
    }
}
