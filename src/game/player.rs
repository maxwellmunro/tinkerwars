use crate::client::programming::CommandSet;
use crate::constants;
use crate::constants::get_component_shape;
use crate::game::component::{Component, ComponentHandle, ComponentKind};
use crate::game::world::World;
use rapier2d::dynamics::{PrismaticJointBuilder, RevoluteJointBuilder, RigidBodyBuilder};
use rapier2d::geometry::ColliderBuilder;
use rapier2d::math::{Isometry, Rotation, Vector};
use rapier2d::na::point;
use rapier2d::pipeline::ActiveHooks;
use rapier2d::prelude::nalgebra;
use rapier2d::prelude::{MotorModel, RigidBodyHandle};
use tokio::sync::RwLockWriteGuard;

pub(crate) struct Player {
    pub components: Vec<ComponentHandle>,
    pub command_set: CommandSet,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            components: Default::default(),
            command_set: CommandSet::new(),
        }
    }
}

impl Player {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            command_set: CommandSet::new(),
        }
    }

    pub fn add_component(
        &mut self,
        world: &mut RwLockWriteGuard<World>,
        kind: ComponentKind,
        mut x: f32,
        mut y: f32,
        rot: f32,
    ) -> Option<(Vec<RigidBodyHandle>, ComponentHandle)> {
        x /= 5.0;
        y /= 5.0;

        let shapes = get_component_shape(kind);

        let body_handles = shapes
            .into_iter()
            .map(|s| {
                let mut body = RigidBodyBuilder::dynamic().build();
                body.set_position(Isometry::new(Vector::new(x, y), 0.0), true);
                body.set_rotation(Rotation::new(rot), true);

                let collider = ColliderBuilder::convex_hull(s)
                    .expect("Component shape must be convex")
                    .active_hooks(ActiveHooks::FILTER_CONTACT_PAIRS)
                    .friction(1.0)
                    .density(0.01)
                    .build();

                let handle = world.bodies.insert(body);
                world.insert(collider, handle);

                world.bodies.get(handle).unwrap().colliders();

                handle
            })
            .collect::<Vec<_>>();

        let joint_handles = match kind {
            ComponentKind::Motor => {
                let motor = RevoluteJointBuilder::new()
                    .local_anchor1(point![0.0, 0.0])
                    .local_anchor2(point![0.0, 0.0])
                    .motor_model(MotorModel::ForceBased)
                    .motor_max_force(constants::MOTOR_MAX_TORQUE)
                    // .motor_velocity(1.0, 100.0)
                    .build();

                let a = body_handles.first()?;
                let b = body_handles.last()?;

                vec![world.impulse_joints.insert(*a, *b, motor, true)]
            }
            ComponentKind::Piston => {
                let piston = PrismaticJointBuilder::new(Vector::x_axis())
                    .local_anchor1(point![0.0, 0.0])
                    .local_anchor2(point![0.0, 0.0])
                    .motor_model(MotorModel::ForceBased)
                    .motor_max_force(constants::PISTON_MAX_FORCE)
                    .limits(constants::PISTON_LIMITS)
                    .build();

                let a = body_handles.first()?;
                let b = body_handles.last()?;

                // let rotation_lock = FixedJointBuilder::new()
                //     .local_anchor1(point![0.0, 0.0])
                //     .local_anchor2(point![0.0, 0.0])
                //     .build();

                vec![
                    world.impulse_joints.insert(*a, *b, piston, true),
                    // world.impulse_joints.insert(*a, *b, rotation_lock, true),
                ]
            }
            _ => vec![],
        };

        let comp = Component::new(body_handles.clone(), joint_handles, kind);
        let handle = world.push_component(comp);
        self.components.push(handle);

        Some((body_handles, handle))
    }
}
