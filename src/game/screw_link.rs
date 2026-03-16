use rapier2d::dynamics::{ImpulseJointHandle, RevoluteJointBuilder, RigidBodySet};
use rapier2d::prelude::{ImpulseJointSet, RigidBodyHandle};

use crate::polygon::Vec2;

pub struct ScrewLink {
    pub a: RigidBodyHandle,
    pub b: RigidBodyHandle,

    pub local_anchor_a: Vec2,
    pub local_anchor_b: Vec2,

    pub joint: ImpulseJointHandle,
}

impl ScrewLink {
    pub fn sync(&mut self, bodies: &mut RigidBodySet, joints: &mut ImpulseJointSet) {
        // let rev = RevoluteJointBuilder::new();
        // let handle = joints.insert(self.a, self.b, rev, true);
        // self.joint = handle;
    }
}
