use core::borrow::Borrow;
use core::ops::{Deref, DerefMut};

use macroquad::color::Color;
use ultraviolet::interp::Lerp;
use ultraviolet::Vec2;

pub const GRAVITY: f32 = 12.0;

#[derive(Clone)]
pub struct Particle {
    pub body: Sphere,
    pub vel: Vec2,
    pub color: Color,
}

impl Particle {
    pub fn update(&mut self, dt: f32) {
        self.pos = self.pos + self.vel * dt;
    }
}

impl Deref for Particle {
    type Target = Sphere;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl DerefMut for Particle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.body
    }
}

pub trait Position {
    fn pos(&self) -> Vec2;
}

pub trait Affect {
    type Subject;

    fn effect_on(&self, other: &Self::Subject) -> Vec2;
}

pub trait Combine {
    fn combine(&mut self, other: &Self);
}

#[derive(Default, Clone)]
pub struct Sphere {
    pub pos: Vec2,
    pub mass: f32,
    pub radius: f32,
}

impl Sphere {
    pub fn new(pos: Vec2, mass: f32, radius: f32) -> Self {
        Self { pos, mass, radius }
    }
}

impl Combine for Sphere {
    fn combine(&mut self, other: &Self) {
        if other.mass > 0.0 {
            self.mass += other.mass;
            self.pos = self.pos.lerp(other.pos, other.mass / self.mass);
        }

        let (r1, r2) = (self.radius, other.radius);
        self.radius = (r1 * r1 * r1 + r2 * r2 * r2).cbrt();
    }
}

impl Position for Sphere {
    fn pos(&self) -> Vec2 {
        self.pos
    }
}

impl Affect for Sphere {
    type Subject = Self;

    fn effect_on(&self, other: &Self::Subject) -> Vec2 {
        if self.mass == 0.0 {
            return Vec2::zero();
        }

        let displacement = self.pos - other.pos;
        let mag_sq = displacement.mag_sq();

        displacement.normalized() * GRAVITY * self.mass * non_neg_softsign(mag_sq / self.radius)
            / (1.0 + mag_sq)
    }
}

impl Borrow<Sphere> for Particle {
    fn borrow(&self) -> &Sphere {
        &self.body
    }
}

const fn non_neg_softsign(x: f32) -> f32 {
    x / (1.0 + x)
}
