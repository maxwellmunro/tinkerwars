#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub const fn new_const(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    } // 90° CCW
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn normalized(self) -> Vec2 {
        let l = self.length();
        if l == 0.0 {
            Vec2::zero()
        } else {
            Vec2::new(self.x / l, self.y / l)
        }
    }
    pub fn mul(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
    pub fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
    pub fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }

    // 2D cross products:
    // scalar cross vector -> Vec2
    pub fn cross_s_v(s: f32, v: Vec2) -> Vec2 {
        Vec2::new(-s * v.y, s * v.x)
    }
    // vector cross scalar -> Vec2
    pub fn cross_v_s(v: Vec2, s: f32) -> Vec2 {
        Vec2::new(s * v.y, -s * v.x)
    }
    // vector cross vector -> scalar
    pub fn cross_v_v(a: Vec2, b: Vec2) -> f32 {
        a.x * b.y - a.y * b.x
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn set_x(&mut self, x: f32) {
        self.x = x;
    }

    pub fn set_y(&mut self, y: f32) {
        self.y = y;
    }
}

// Simple utility: clamp value between min and max
pub fn clamp(x: f32, min: f32, max: f32) -> f32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pos: Vec2,
    weight: f32, // per-vertex weight
}

impl Vertex {
    pub const fn new_const(x: f32, y: f32, weight: f32) -> Self {
        Vertex {
            pos: Vec2::new_const(x, y),
            weight,
        }
    }

    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }

    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos = pos;
    }

    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
    }
}

#[derive(Debug)]
pub struct Body {
    // shape described by vertices in local space (relative to some local origin)
    vertices: Vec<Vertex>,

    // dynamic properties (world-space)
    position: Vec2,        // center of mass position in world coords
    orientation: f32,      // rotation angle (radians)
    velocity: Vec2,        // linear velocity
    angular_velocity: f32, // scalar angular velocity (radians/sec)

    // mass properties (computed from vertex weights)
    mass: f32,
    inv_mass: f32,
    inertia: f32,
    inv_inertia: f32,

    // accumulators
    force: Vec2,
    torque: f32,
}

impl Body {
    pub fn new(vertices: Vec<Vertex>, position: Vec2, orientation: f32) -> Self {
        let mut b = Self {
            vertices,
            position,
            orientation,
            velocity: Vec2::zero(),
            angular_velocity: 0.0,
            mass: 1.0,
            inv_mass: 1.0,
            inertia: 1.0,
            inv_inertia: 1.0,
            force: Vec2::zero(),
            torque: 0.0,
        };
        b.compute_mass_properties();
        b
    }

    // Compute mass, COM (and shift vertices so local COM is at origin), and moment of inertia
    // Using per-vertex weights: mass = sum weights, COM = weighted average,
    // inertia = sum w * r^2 (point mass approximation)
    pub fn compute_mass_properties(&mut self) {
        let total_mass: f32 = self.vertices.iter().map(|v| v.weight).sum();
        if total_mass <= 0.0 {
            self.mass = 1.0;
            self.inv_mass = 1.0;
            self.inertia = 1.0;
            self.inv_inertia = 1.0;
            return;
        }

        // compute COM in local coordinates (vertices are currently in local coords)
        let com = {
            let mut acc = Vec2::zero();
            for v in &self.vertices {
                acc = acc.add(v.pos.mul(v.weight));
            }
            acc.mul(1.0 / total_mass)
        };

        // shift vertices so COM is at (0,0) local
        for v in &mut self.vertices {
            v.pos = v.pos.sub(com);
        }

        // compute inertia about local COM (point masses)
        let mut inertia = 0.0f32;
        for v in &self.vertices {
            let r2 = v.pos.x * v.pos.x + v.pos.y * v.pos.y;
            inertia += v.weight * r2;
        }

        self.mass = total_mass;
        self.inv_mass = if total_mass > 0.0 {
            1.0 / total_mass
        } else {
            0.0
        };
        self.inertia = inertia.max(1e-6); // avoid zero inertia
        self.inv_inertia = 1.0 / self.inertia;
    }

    // Convert a point in local space to world space
    pub fn local_to_world(&self, local: Vec2) -> Vec2 {
        // rotation by orientation then translate
        let c = self.orientation.cos();
        let s = self.orientation.sin();
        let x = local.x * c - local.y * s;
        let y = local.x * s + local.y * c;
        Vec2::new(x, y).add(self.position)
    }

    // Get world-space position of COM (self.position is already COM)
    pub fn world_com(&self) -> Vec2 {
        self.position
    }

    // Integrate forces to update velocity (semi-implicit Euler)
    pub fn integrate_forces(&mut self, dt: f32) {
        if self.inv_mass == 0.0 {
            return;
        } // static body
        let accel = self.force.mul(self.inv_mass);
        self.velocity = self.velocity.add(accel.mul(dt));
        self.angular_velocity += self.torque * self.inv_inertia * dt;
        // reset accumulators (in a real engine you might keep gravity)
        self.force = Vec2::zero();
        self.torque = 0.0;
    }

    // Integrate velocities to update positions
    pub fn integrate_velocity(&mut self, dt: f32) {
        if self.inv_mass == 0.0 {
            return;
        }
        self.position = self.position.add(self.velocity.mul(dt));
        self.orientation += self.angular_velocity * dt;
    }

    // Computes world-space velocity at a given world-space point p (v + w x r)
    pub fn velocity_at_world_point(&self, p: Vec2) -> Vec2 {
        let r = p.sub(self.world_com());
        // w x r (2D): (-w * r.y, w * r.x)
        let ang = Vec2::new(-self.angular_velocity * r.y, self.angular_velocity * r.x);
        self.velocity.add(ang)
    }

    // apply linear and angular impulse at world point p
    pub fn apply_impulse(&mut self, impulse: Vec2, contact_point: Vec2) {
        if self.inv_mass == 0.0 {
            return;
        }
        self.velocity = self.velocity.add(impulse.mul(self.inv_mass));
        let r = contact_point.sub(self.world_com());
        let delta_ang = Vec2::cross_v_v(r, impulse) * self.inv_inertia;
        self.angular_velocity += delta_ang;
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn orientation(&self) -> f32 {
        self.orientation
    }

    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }

    pub fn angular_velocity(&self) -> f32 {
        self.angular_velocity
    }

    pub fn mass(&self) -> f32 {
        self.mass
    }

    pub fn inv_mass(&self) -> f32 {
        self.inv_mass
    }

    pub fn inertia(&self) -> f32 {
        self.inertia
    }

    pub fn inv_inertia(&self) -> f32 {
        self.inv_inertia
    }

    pub fn force(&self) -> Vec2 {
        self.force
    }

    pub fn torque(&self) -> f32 {
        self.torque
    }

    pub fn set_vertices(&mut self, vertices: Vec<Vertex>) {
        self.vertices = vertices;
        self.compute_mass_properties();
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }

    pub fn set_orientation(&mut self, orientation: f32) {
        self.orientation = orientation;
    }

    pub fn set_velocity(&mut self, velocity: Vec2) {
        self.velocity = velocity;
    }

    pub fn set_angular_velocity(&mut self, angular_velocity: f32) {
        self.angular_velocity = angular_velocity;
    }

    pub fn set_force(&mut self, force: Vec2) {
        self.force = force;
    }

    pub fn set_torque(&mut self, torque: f32) {
        self.torque = torque;
    }
}

#[derive(Debug)]
struct Contact {
    point: Vec2,      // world-space contact point
    normal: Vec2,     // unit normal pointing from A -> B
    penetration: f32, // positive penetration depth
    restitution: f32, // e (0..1)
    friction: f32,    // mu
}

// positional correction to remove penetration bias (Baumgarte-like)
fn positional_correction(a: &mut Body, b: &mut Body, contact: &Contact) {
    let percent = 0.8; // positional correction factor
    let slop = 0.01; // penetration allowance
    let penetration = (contact.penetration - slop).max(0.0);
    if penetration <= 0.0 {
        return;
    }

    let inv_mass_sum = a.inv_mass + b.inv_mass;
    if inv_mass_sum == 0.0 {
        return;
    }

    let correction_mag = (penetration / inv_mass_sum) * percent as f32;
    let correction = contact.normal.mul(correction_mag);

    // Move A opposite to normal, B along normal
    a.position = a.position.sub(correction.mul(a.inv_mass));
    b.position = b.position.add(correction.mul(b.inv_mass));
}

// Resolve a single contact between body A and body B.
// Assumes contact normal points from A -> B.
fn resolve_contact(a: &mut Body, b: &mut Body, contact: &Contact) {
    // 1) compute relative velocity at contact
    let ra = contact.point.sub(a.world_com());
    let rb = contact.point.sub(b.world_com());

    let va = a.velocity_at_world_point(contact.point);
    let vb = b.velocity_at_world_point(contact.point);
    let rel = vb.sub(va);

    // 2) relative velocity along normal
    let vel_along_normal = rel.dot(contact.normal);

    // if velocities are separating, don't resolve (but might still need positional correction)
    if vel_along_normal > 0.0 {
        return;
    }

    // 3) compute scalar impulse magnitude (normal)
    let e = contact.restitution;

    // rotational contributions: (r x n)^2 / I
    let ra_cross_n = Vec2::cross_v_v(ra, contact.normal);
    let rb_cross_n = Vec2::cross_v_v(rb, contact.normal);
    let inv_inertia_term =
        ra_cross_n * ra_cross_n * a.inv_inertia + rb_cross_n * rb_cross_n * b.inv_inertia;

    let denom = a.inv_mass + b.inv_mass + inv_inertia_term;
    let j = if denom > 0.0 {
        -(1.0 + e) * vel_along_normal / denom
    } else {
        0.0
    };

    // apply normal impulse
    let impulse = contact.normal.mul(j);
    a.apply_impulse(impulse.mul(-1.0), contact.point); // A gets -j*n
    b.apply_impulse(impulse, contact.point); // B gets +j*n

    // 4) friction impulse (Coulomb)
    // compute relative velocity after normal impulse
    let va2 = a.velocity_at_world_point(contact.point);
    let vb2 = b.velocity_at_world_point(contact.point);
    let rel2 = vb2.sub(va2);

    // tangent (relative velocity minus normal component)
    let tangent = (rel2.sub(contact.normal.mul(rel2.dot(contact.normal)))).normalized();
    let vt = rel2.dot(tangent);

    let ra_cross_t = Vec2::cross_v_v(ra, tangent);
    let rb_cross_t = Vec2::cross_v_v(rb, tangent);
    let inv_inertia_t =
        ra_cross_t * ra_cross_t * a.inv_inertia + rb_cross_t * rb_cross_t * b.inv_inertia;

    let denom_t = a.inv_mass + b.inv_mass + inv_inertia_t;
    if denom_t == 0.0 {
        return;
    }

    let mut jt = -vt / denom_t;
    let mu = contact.friction;
    // clamp jt by Coulomb: |jt| <= mu * j_normal
    let max_jt = j.abs() * mu;
    jt = clamp(jt, -max_jt, max_jt);

    let friction_impulse = tangent.mul(jt);
    a.apply_impulse(friction_impulse.mul(-1.0), contact.point);
    b.apply_impulse(friction_impulse, contact.point);
}
