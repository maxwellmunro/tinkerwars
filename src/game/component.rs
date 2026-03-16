use crate::constants;
use crate::constants::{get_component_health, get_component_texture};
use crate::polygon::Vec2;
use crate::texture_handler::TextureHandler;
use bincode::{Decode, Encode};
use rapier2d::dynamics::{ImpulseJointHandle, ImpulseJointSet, JointAxis, RigidBodySet};
use rapier2d::prelude::RigidBodyHandle;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::cmp::PartialEq;
use std::f32::consts::PI;

#[derive(PartialEq, Clone, Copy, Debug, Default, Encode, Decode)]
pub(crate) enum ComponentActivationState {
    #[default]
    None,
    Spinning,
    ReverseSpin,
    NotSpinning,
    Extending,
    Retracting,
    PitonStopped,
}

#[derive(PartialEq, Clone, Copy, Debug, Eq, Hash, Encode, Decode)]
pub(crate) enum ComponentKind {
    ArmLarge,
    ArmMedium,
    ArmSmall,
    ArmTiny,
    Body,
    Motor,
    Piston,
    Screw,
}

#[derive(Clone, Debug, Encode, Decode)]
pub(in crate::game) struct ComponentSave {
    pub(in crate::game) component_kind: ComponentKind,
    pub(in crate::game) health: f32,
    pub(in crate::game) pos: Vec2,
    pub(in crate::game) rot: f32,
    pub(in crate::game) activated: ComponentActivationState,
    /// Motor velocity / piston extension
    pub(in crate::game) target_amount: f32,
    pub(in crate::game) piston_speed: f32,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Encode, Decode)]
pub(crate) struct ComponentHandle(pub u64);

#[derive(Clone, Debug)]
pub(crate) struct Component {
    pub(in crate::game) save: ComponentSave,
    pub(in crate::game) bodies: Vec<RigidBodyHandle>,
    pub(in crate::game) joints: Vec<ImpulseJointHandle>,
}

impl Component {
    pub(in crate::game) fn new(
        bodies: Vec<RigidBodyHandle>,
        joints: Vec<ImpulseJointHandle>,
        kind: ComponentKind,
    ) -> Self {
        Self {
            save: ComponentSave {
                component_kind: kind,
                health: get_component_health(kind),
                pos: Vec2::default(),
                rot: 0.0,
                activated: if kind == ComponentKind::Motor {
                    ComponentActivationState::Spinning
                } else if kind == ComponentKind::Piston {
                    ComponentActivationState::Extending
                } else {
                    Default::default()
                },
                target_amount: 5.0,
                piston_speed: 2.0,
            },
            bodies,
            joints,
        }
    }

    pub(in crate::game) fn render(
        &self,
        bodies: &RigidBodySet,
        texture_handler: &TextureHandler,
        canvas: &mut Canvas<Window>,
    ) -> Result<(), String> {
        let tex = texture_handler.get_texture(get_component_texture(self.save.component_kind));

        match self.save.component_kind {
            ComponentKind::Motor => {
                let dst = Rect::new(
                    (self.save.pos.x * constants::PIXELS_PER_METER - tex.1.0 as f32 / 4.0) as i32,
                    (self.save.pos.y * constants::PIXELS_PER_METER - tex.1.1 as f32 / 2.0) as i32,
                    tex.1.0 / 2,
                    tex.1.1,
                );
                let center = Point::new(dst.width() as i32 / 2, dst.height() as i32 / 2);

                let a = bodies.get(*self.bodies.first().unwrap()).unwrap();
                let b = bodies.get(*self.bodies.last().unwrap()).unwrap();

                let mut src = Rect::new(0, 0, 16, 16);
                canvas.copy_ex(
                    tex.0,
                    Some(src),
                    Some(dst),
                    (a.rotation().angle() * 180.0 / PI) as f64,
                    center,
                    false,
                    false,
                )?;

                src.set_x(src.width() as i32);
                canvas.copy_ex(
                    tex.0,
                    Some(src),
                    Some(dst),
                    (b.rotation().angle() * 180.0 / PI) as f64,
                    center,
                    false,
                    false,
                )?;
            }
            ComponentKind::Piston => {
                let a = bodies.get(*self.bodies.first().unwrap()).unwrap();
                let b = bodies.get(*self.bodies.last().unwrap()).unwrap();

                let x_a = a.position().translation.x;
                let y_a = a.position().translation.y;

                let x_b = b.position().translation.x;
                let y_b = b.position().translation.y;

                let mut dst = Rect::new(
                    (x_b * constants::PIXELS_PER_METER - tex.1.0 as f32 / 2.0) as i32,
                    (y_b * constants::PIXELS_PER_METER - tex.1.1 as f32 / 4.0) as i32,
                    tex.1.0,
                    tex.1.1 / 2,
                );
                let center = Point::new(dst.width() as i32 / 2, dst.height() as i32 / 2);

                let mut src = Rect::new(0, 8, 32, 8);

                src.set_y(src.height() as i32);
                canvas.copy_ex(
                    tex.0,
                    Some(src),
                    Some(dst),
                    (b.rotation().angle() * 180.0 / PI) as f64,
                    center,
                    false,
                    false,
                )?;

                canvas.copy_ex(
                    tex.0,
                    Some(src),
                    Some(dst),
                    (a.rotation().angle() * 180.0 / PI) as f64,
                    center,
                    false,
                    false,
                )?;

                dst.set_x((x_a * constants::PIXELS_PER_METER as f32 - tex.1.0 as f32 / 2.0) as i32);
                dst.set_y((y_a * constants::PIXELS_PER_METER as f32 - tex.1.1 as f32 / 4.0) as i32);

                src.set_y(0);
                canvas.copy_ex(
                    tex.0,
                    Some(src),
                    Some(dst),
                    (b.rotation().angle() * 180.0 / PI) as f64,
                    center,
                    false,
                    false,
                )?;
            }
            _ => {
                let dst = Rect::new(
                    (self.save.pos.x * constants::PIXELS_PER_METER - tex.1.0 as f32 / 2.0) as i32,
                    (self.save.pos.y * constants::PIXELS_PER_METER - tex.1.1 as f32 / 2.0) as i32,
                    tex.1.0,
                    tex.1.1,
                );
                let center = Point::new(dst.width() as i32 / 2, dst.height() as i32 / 2);

                canvas.copy_ex(
                    tex.0,
                    None,
                    Some(dst),
                    (self.save.rot * 180.0 / PI) as f64,
                    center,
                    false,
                    false,
                )?;
            }
        };

        Ok(())
    }

    pub(in crate::game) fn tick(&mut self, joints: &mut ImpulseJointSet) {
        match self.save.component_kind {
            ComponentKind::Motor => self.tick_motor(joints),
            ComponentKind::Piston => self.tick_piston(joints),
            _ => {}
        }
    }

    pub(in crate::game) fn bodies(&self) -> &Vec<RigidBodyHandle> {
        &self.bodies
    }

    fn tick_motor(&mut self, joints: &mut ImpulseJointSet) {
        let joint = joints
            .get_mut(*self.joints.first().unwrap(), false)
            .unwrap();

        joint.data.set_motor_velocity(
            JointAxis::AngX,
            match self.save.activated {
                ComponentActivationState::Spinning => self.save.target_amount,
                ComponentActivationState::ReverseSpin => -self.save.target_amount,
                _ => 0.0,
            },
            10.0,
        );
    }

    fn tick_piston(&mut self, joints: &mut ImpulseJointSet) {
        let joint = joints
            .get_mut(*self.joints.first().unwrap(), false)
            .unwrap();

        joint.data.set_motor_velocity(
            JointAxis::LinX,
            match self.save.activated {
                ComponentActivationState::Extending => self.save.piston_speed,
                ComponentActivationState::Retracting => -self.save.piston_speed,
                _ => 0.0,
            },
            10.0,
        );
    }

    pub fn set_state(&mut self, state: ComponentActivationState) {
        self.save.activated = state;
    }
}

pub(crate) fn render_component(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    component_kind: ComponentKind,
    x: i32,
    y: i32,
    rot: f32,
    scale: f32,
    selected: bool,
) -> Result<(), String> {
    let mut tex = if selected {
        texture_handler.get_texture(constants::get_mask_texture(component_kind))
    } else {
        texture_handler.get_texture(constants::get_component_texture(component_kind))
    };

    tex.1.0 = (tex.1.0 as f32 * scale) as u32;
    tex.1.1 = (tex.1.1 as f32 * scale) as u32;

    match component_kind {
        ComponentKind::Motor => {
            let dst = Rect::new(
                (x as f32 - tex.1.0 as f32 / 4.0) as i32,
                (y as f32 - tex.1.1 as f32 / 2.0) as i32,
                tex.1.0 / 2,
                tex.1.1,
            );
            let center = Point::new(dst.width() as i32 / 2, dst.height() as i32 / 2);

            let mut src = Rect::new(0, 0, 16, 16);
            canvas.copy_ex(
                tex.0,
                Some(src),
                Some(dst),
                (rot * 180.0 / PI) as f64,
                center,
                false,
                false,
            )?;

            src.set_x(src.width() as i32);
            canvas.copy_ex(
                tex.0,
                Some(src),
                Some(dst),
                (rot * 180.0 / PI) as f64,
                center,
                false,
                false,
            )?;
        }
        ComponentKind::Piston => {
            let mut dst = Rect::new(
                (x as f32 - tex.1.0 as f32 / 2.0) as i32,
                (y as f32 - tex.1.1 as f32 / 4.0) as i32,
                tex.1.0,
                tex.1.1 / 2,
            );
            let center = Point::new(dst.width() as i32 / 2, dst.height() as i32 / 2);

            let mut src = Rect::new(0, 8, 32, 8);

            src.set_y(src.height() as i32);
            canvas.copy_ex(
                tex.0,
                Some(src),
                Some(dst),
                (rot * 180.0 / PI) as f64,
                center,
                false,
                false,
            )?;

            canvas.copy_ex(
                tex.0,
                Some(src),
                Some(dst),
                (rot * 180.0 / PI) as f64,
                center,
                false,
                false,
            )?;

            dst.set_x((x as f32 - tex.1.0 as f32 / 2.0) as i32);
            dst.set_y((y as f32 - tex.1.1 as f32 / 4.0) as i32);

            src.set_y(0);
            canvas.copy_ex(
                tex.0,
                Some(src),
                Some(dst),
                (rot * 180.0 / PI) as f64,
                center,
                false,
                false,
            )?;
        }
        _ => {
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
        }
    };

    Ok(())
}
