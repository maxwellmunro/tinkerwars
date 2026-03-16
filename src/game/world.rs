use crate::constants;
use crate::game::component::{Component, ComponentHandle};
use crate::game::player::Player;
use crate::game::screw_link::ScrewLink;
use crate::polygon::Vec2;
use crate::texture_handler::{TextureHandler, TextureId};
use rapier2d::dynamics::{
    CCDSolver, ImpulseJointHandle, ImpulseJointSet, IntegrationParameters, IslandManager,
    MultibodyJointSet, RevoluteJointBuilder, RigidBodyHandle, RigidBodySet,
};
use rapier2d::geometry::{Collider, ColliderHandle, ColliderSet, SolverFlags};
use rapier2d::math::Vector;
use rapier2d::na::point;
use rapier2d::pipeline::{PhysicsHooks, PhysicsPipeline};
use rapier2d::prelude::BroadPhaseBvh;
use rapier2d::prelude::{DefaultBroadPhase, NarrowPhase};
use rapier2d::prelude::{PairFilterContext, nalgebra};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;

#[derive(Default)]
pub struct PairCollisionFilter {
    pub disabled_pairs: HashSet<(ColliderHandle, ColliderHandle)>,
}

impl PhysicsHooks for PairCollisionFilter {
    fn filter_contact_pair(&self, context: &PairFilterContext<'_>) -> Option<SolverFlags> {
        let a = context.collider1;
        let b = context.collider2;

        if self.disabled_pairs.contains(&(a, b)) || self.disabled_pairs.contains(&(b, a)) {
            return None;
        }

        Some(SolverFlags::COMPUTE_IMPULSES)
    }
}

#[derive(Default)]
pub(crate) struct ComponentSet {
    components: HashMap<ComponentHandle, Component>,
    next_id: u64,
}

impl ComponentSet {
    pub fn new() -> Self {
        ComponentSet {
            components: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn get(&self, handle: ComponentHandle) -> Option<&Component> {
        self.components.get(&handle)
    }

    pub fn get_mut(&mut self, handle: ComponentHandle) -> Option<&mut Component> {
        self.components.get_mut(&handle)
    }

    pub fn push(&mut self, component: Component) -> ComponentHandle {
        let handle = ComponentHandle(self.next_id);
        self.next_id += 1;

        self.components.insert(handle, component);
        handle
    }

    pub fn remove(&mut self, handle: ComponentHandle) -> Option<Component> {
        self.components.remove(&handle)
    }
}

pub enum KeyState {
    JustPressed,
    JustReleased,
    Pressed,
    Unpressed,
}

#[derive(Default)]
pub struct World {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,

    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,

    pub pipeline: PhysicsPipeline,
    pub gravity: Vector<f32>,
    pub integration: IntegrationParameters,
    pub islands: IslandManager,
    pub broad_phase: BroadPhaseBvh,
    pub narrow_phase: NarrowPhase,
    pub ccd: CCDSolver,

    pub filter: PairCollisionFilter,

    pub players: Vec<Player>,
    pub me: Player,
    pub components: ComponentSet,
    pub screws: Vec<ScrewLink>,
    
    pub keys: HashMap<Keycode, KeyState>,
}

impl World {
    pub fn new() -> Self {
        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),

            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),

            pipeline: PhysicsPipeline::new(),
            gravity: Vector::new(0.0, 20.0),
            integration: IntegrationParameters::default(),
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd: CCDSolver::new(),

            filter: PairCollisionFilter {
                disabled_pairs: HashSet::new(),
            },

            players: Vec::new(),
            me: Player::new(),
            components: ComponentSet::new(),
            screws: Vec::new(),

            keys: HashMap::new(),
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        texture_handler: &TextureHandler,
    ) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(100, 100, 255));

        let screw_tex = texture_handler.get_texture(TextureId::ComponentScrew).0;

        self.players.iter().try_for_each(|p| {
            p.components.iter().try_for_each(|c| {
                let rotate = |p: Vec2, a: f32| {
                    let (x, y) = (p.x, p.y);

                    let mag = (x * x + y * y).sqrt();
                    let angle = y.atan2(x) + a;

                    Vec2 {
                        x: mag * angle.cos(),
                        y: mag * angle.sin(),
                    }
                };

                let Some(c) = &self.components.get(*c) else {
                    return Err(String::from("Null pointing component handle?"));
                };

                if false {
                    for (_, collider) in self.colliders.iter() {
                        let component_bodies = c.bodies();
                        for body in component_bodies {
                            let Some(rigid_body) = self.bodies.get(*body) else {
                                continue;
                            };

                            if collider.parent().map(|p| p) == Some(*body) {
                                if let Some(p) = collider.shape().as_convex_polygon() {
                                    let points = p
                                        .points()
                                        .iter()
                                        .map(|p| {
                                            let rp = rotate(
                                                Vec2 { x: p.x, y: p.y },
                                                rigid_body.rotation().angle(),
                                            );
                                            rp.add(Vec2 {
                                                x: rigid_body.position().translation.x,
                                                y: rigid_body.position().translation.y,
                                            })
                                            .mul(constants::PIXELS_PER_METER)
                                            .into()
                                        })
                                        .collect::<Vec<_>>();

                                    canvas.draw_lines(points.as_slice())?;
                                }
                            }
                        }
                    }

                    Ok::<(), String>(())
                } else {
                    c.render(&self.bodies, texture_handler, canvas)
                }?;

                c.joints.iter().try_for_each(|s| {
                    let Some(s) = self.impulse_joints.get(*s) else {
                        return Err(String::from("Impulse joint not found?"));
                    };

                    if s.data.user_data != 1 {
                        return Ok(());
                    }

                    let handle1 = s.body1;
                    let handle2 = s.body2;
                    let Some(body1) = self.bodies.get(handle1) else {
                        return Err(String::from("No body found for anchor?"));
                    };
                    let Some(body2) = self.bodies.get(handle2) else {
                        return Err(String::from("No body found for anchor?"));
                    };

                    let w_pos1 = body1.position() * s.data.local_anchor1();
                    let w_pos2 = body2.position() * s.data.local_anchor2();

                    let angle = 90.0 * (body1.rotation().angle() + body2.rotation().angle()) / PI;

                    let a_w_pos = Vec2 {
                        x: (w_pos1.x + w_pos2.x) / 2.0,
                        y: (w_pos1.y + w_pos2.y) / 2.0,
                    };

                    let rect = Rect::new( (a_w_pos.x * constants::PIXELS_PER_METER - 6.0) as i32,
                        (a_w_pos.y * constants::PIXELS_PER_METER - 6.0) as i32,
                        12,
                        12,
                    );

                    canvas.copy_ex(
                        screw_tex,
                        None,
                        Some(rect),
                        angle as f64,
                        Point::new(6, 6),
                        false,
                        false,
                    )
                })
            })
        })?;

        Ok(())
    }

    pub fn tick(&mut self, mut dt: f32) {
        dt /= constants::PHYSICS_STEPS as f32;

        for _ in 0..constants::PHYSICS_STEPS {
            self.components.components.iter_mut().for_each(|(_, c)| {
                c.tick(&mut self.impulse_joints);
            });

            self.integration.dt = dt;

            self.pipeline.step(
                &self.gravity,
                &self.integration,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd,
                &self.filter as &dyn PhysicsHooks,
                &(),
            );

            for player in &mut self.players {
                for comp in &mut player.components {
                    Self::update_component(&mut self.components, *comp, &self.bodies);
                }
            }

            for screw in &mut self.screws {
                screw.sync(&mut self.bodies, &mut self.impulse_joints);
            }
        }
    }

    fn update_component(
        components: &mut ComponentSet,
        comp: ComponentHandle,
        bodies: &RigidBodySet,
    ) {
        let Some(comp) = &mut components.get_mut(comp) else {
            return;
        };

        let component_bodies = comp.bodies().clone();

        for body in component_bodies {
            if let Some(body) = bodies.get(body) {
                let pos = body.translation();
                let rot = body.rotation().angle();

                comp.save.pos = Vec2 { x: pos.x, y: pos.y };
                comp.save.rot = rot;
            }
        }
    }

    pub fn insert(&mut self, collider: Collider, handle: RigidBodyHandle) {
        self.colliders
            .insert_with_parent(collider, handle, &mut self.bodies);
    }

    pub fn make_screw(
        &mut self,
        a: ComponentHandle,
        ia: u64,
        b: ComponentHandle,
        ib: u64,
        mut anchor_world: Vec2,
    ) -> ImpulseJointHandle {
        anchor_world = anchor_world.mul(0.2);

        let c_a = self.components.get(a).unwrap();
        let c_b = self.components.get(b).unwrap();

        let h_a = c_a.bodies[ia as usize];
        let h_b = c_b.bodies[ib as usize];

        let a_rb = self.bodies.get(h_a).unwrap();
        let b_rb = self.bodies.get(h_b).unwrap();

        let world_pt = point![anchor_world.x, anchor_world.y];

        let local_a = a_rb.position().inverse_transform_point(&world_pt);
        let local_b = b_rb.position().inverse_transform_point(&world_pt);

        let mut joint = RevoluteJointBuilder::new()
            .local_anchor1(local_a.clone())
            .local_anchor2(local_b.clone())
            .build();

        joint.data.user_data = 1;

        let handle = self.impulse_joints.insert(h_a, h_b, joint, true);

        self.components.get_mut(a).unwrap().joints.push(handle);
        self.components.get_mut(b).unwrap().joints.push(handle);

        self.disable_body_collision(h_a, h_b);

        self.screws.push(ScrewLink {
            a: h_a,
            b: h_b,
            local_anchor_a: local_a.into(),
            local_anchor_b: local_b.into(),
            joint: handle,
        });

        handle
    }

    pub fn disable_body_collision(&mut self, a: RigidBodyHandle, b: RigidBodyHandle) {
        let body_a = match self.bodies.get(a) {
            Some(b) => b,
            None => return,
        };
        let body_b = match self.bodies.get(b) {
            Some(b) => b,
            None => return,
        };

        for &ch_a in body_a.colliders() {
            for &ch_b in body_b.colliders() {
                self.filter.disabled_pairs.insert((ch_a, ch_b));
            }
        }
    }

    pub fn enable_body_collision(&mut self, a: RigidBodyHandle, b: RigidBodyHandle) {
        if let (Some(body_a), Some(body_b)) = (self.bodies.get(a), self.bodies.get(b)) {
            for &ch_a in body_a.colliders() {
                for &ch_b in body_b.colliders() {
                    self.filter.disabled_pairs.remove(&(ch_a, ch_b));
                    self.filter.disabled_pairs.remove(&(ch_b, ch_a));
                }
            }
        }
    }

    pub fn push_component(&mut self, comp: Component) -> ComponentHandle {
        self.components.push(comp)
    }
}
