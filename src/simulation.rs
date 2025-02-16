use core::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI};

use crate::{
    physics::{Particle, Sphere, GRAVITY},
    space::{Quadtree, Reactions},
};

use macroquad::math::FloatExt;
use macroquad::{color::*, shapes::*, text::draw_text, time::*, window::*};
use miniquad::window::screen_size;
use ultraviolet::{Rotor2, Vec2};

pub struct Simulation {
    particles: Vec<Particle>,
    quadtree: Quadtree,
    reactions: Reactions<Sphere>,
    field: Vec<Vec2>,
    fps: i32,
    logged: u32,
}

fn create_system_around(
    center: Particle,
    count: usize,
    radius: f32,
    cw: bool,
    color: Color,
) -> impl Iterator<Item = Particle> {
    let mut distances = Vec::new();

    distances.resize_with(count, || {
        fastrand::f32()
            .remap(0.0, 1.0, center.radius * 2.0 / radius, 1.0)
            .sqrt()
            * radius
    });

    distances.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

    let mut inner_mass = center.mass;
    let system_rotation = Rotor2::from_angle(if cw { 1.0 } else { -1.0 } * FRAC_PI_2);

    distances.into_iter().map(move |distance| {
        let mass = fastrand::f32().remap(0.0, 1.0, 0.25, 2.0);
        let radius = f32::cbrt(0.75 * mass / PI);

        let angle = fastrand::f32() * core::f32::consts::TAU;
        let pos = Vec2::unit_x().rotated_by(Rotor2::from_angle(angle));
        let vel = pos.rotated_by(system_rotation) * f32::sqrt(GRAVITY * inner_mass / distance);

        inner_mass += mass;

        Particle {
            body: Sphere::new(center.pos + distance * pos, mass, radius),
            vel: center.vel + vel,
            color,
        }
    })
}

impl Simulation {
    pub fn init() -> Self {
        const N: usize = 10000;

        let mut particles = Vec::with_capacity(N);

        // Star 1
        particles.push(Particle {
            body: Sphere::new(Vec2::new(-100.0, 0.0), 25000.0, 12.0),
            vel: Vec2::new(10.0, -10.0),
            color: WHITE,
        });

        // Star 2
        particles.push(Particle {
            body: Sphere::new(Vec2::new(100.0, 0.0), 10000.0, 8.0),
            vel: Vec2::new(-10.0, 30.0),
            color: WHITE,
        });

        // System 1
        particles.extend(create_system_around(
            particles[0].clone(),
            N * 3 / 4,
            screen_height() / 10.0,
            true,
            Color::from_rgba(100, 255, 255, 100),
        ));

        // System 2
        particles.extend(create_system_around(
            particles[1].clone(),
            N * 1 / 4,
            screen_height() / 30.0,
            false,
            Color::from_rgba(255, 125, 0, 100),
        ));

        // Draw larger particles on top
        particles.sort_unstable_by(|a, b| a.mass.partial_cmp(&b.mass).unwrap());
        let field = vec![Vec2::zero(); particles.len()];

        Self {
            particles,
            quadtree: Quadtree::new(Vec2::zero(), 1.0),
            reactions: Reactions::new(),
            field,
            fps: 0,
            logged: 0,
        }
    }

    pub fn update(&mut self) {
        clear_background(BLACK);

        // Remove far away particles
        self.particles.retain(|particle| particle.pos.mag() < 400.0);

        // Compute bounding box
        let aabb = crate::space::aabb(self.particles.iter().map(|object| object.pos));

        // Build quadtree
        // let time = Instant::now();
        self.quadtree
            .clear()
            .set_pos(aabb.0)
            .set_size(aabb.1.component_max() + 1.0)
            .build_from_objects::<_, Sphere>(&self.particles, 0..self.particles.len());
        // println!("build: {:?}", time.elapsed());

        // Compute field interactions
        // let time = Instant::now();
        self.field.fill(Vec2::zero());

        self.reactions
            .clear()
            .compute_on(&self.particles, &mut self.quadtree, &mut self.field);
        // println!("attract: {:?}", time.elapsed());

        // println!("");

        for i in 0..self.particles.len() {
            self.particles[i].vel += self.field[i] * get_frame_time();
        }

        // Update particles
        for particle in &mut self.particles {
            particle.update(get_frame_time());
        }

        // Update fps around 10 times per second
        let time = (10.0 * get_time()) as u32;

        if time > self.logged {
            self.logged = time;
            self.fps = get_fps();
        }
    }

    pub fn render(&self) {
        let screen_size = Vec2::from(screen_size()) / screen_dpi_scale();
        let center = screen_size * 0.5;

        for particle in self.particles.iter() {
            let r = particle.radius * FRAC_1_SQRT_2;

            draw_rectangle(
                center.x + particle.pos.x - r,
                center.y + particle.pos.y - r,
                r * 2.0,
                r * 2.0,
                particle.color,
            );
        }

        draw_text(&format!("{}", self.fps), 10.0, 16.0, 12.0, WHITE);
    }
}
