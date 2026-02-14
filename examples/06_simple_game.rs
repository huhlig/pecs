//! Simple Game Example - Asteroids-style
//!
//! This example demonstrates building a complete game simulation using PECS.
//! It showcases:
//! - Multiple component types working together
//! - Query system for game logic
//! - Component mutation during gameplay
//! - Entity lifecycle management
//! - Collision detection patterns
//!
//! Run with: cargo run --example 06_simple_game --release

use pecs::prelude::*;
use std::f32::consts::PI;

// ============================================================================
// Components
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
struct Rotation {
    angle: f32, // radians
}

impl Component for Rotation {}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: i32,
    #[allow(dead_code)]
    max: i32,
}

impl Component for Health {}

#[derive(Debug, Clone, Copy)]
struct Damage {
    #[allow(dead_code)]
    amount: i32,
}

impl Component for Damage {}

#[derive(Debug, Clone, Copy)]
struct Radius {
    value: f32,
}

impl Component for Radius {}

// Entity type markers
#[derive(Debug, Clone, Copy)]
struct Player;
impl Component for Player {}

#[derive(Debug, Clone, Copy)]
struct Asteroid;
impl Component for Asteroid {}

#[derive(Debug, Clone, Copy)]
struct Bullet;
impl Component for Bullet {}

// ============================================================================
// Game State
// ============================================================================

struct Game {
    world: World,
    frame: u64,
    score: u32,
}

impl Game {
    fn new() -> Self {
        let mut world = World::with_capacity(1000);

        // Spawn player
        world
            .spawn()
            .with(Player)
            .with(Position { x: 0.0, y: 0.0 })
            .with(Velocity { x: 0.0, y: 0.0 })
            .with(Rotation { angle: 0.0 })
            .with(Health {
                current: 100,
                max: 100,
            })
            .with(Radius { value: 10.0 })
            .id();

        // Spawn initial asteroids
        for i in 0..10 {
            let angle = (i as f32) * (2.0 * PI / 10.0);
            let distance = 200.0;
            world
                .spawn()
                .with(Asteroid)
                .with(Position {
                    x: angle.cos() * distance,
                    y: angle.sin() * distance,
                })
                .with(Velocity {
                    x: angle.cos() * -20.0,
                    y: angle.sin() * -20.0,
                })
                .with(Health { current: 3, max: 3 })
                .with(Radius { value: 20.0 })
                .id();
        }

        Self {
            world,
            frame: 0,
            score: 0,
        }
    }

    fn update(&mut self, dt: f32) {
        self.frame += 1;

        // Movement system - update positions based on velocity
        self.movement_system(dt);

        // Collision system - detect and handle collisions
        self.collision_system();

        // Cleanup system - remove dead entities
        self.cleanup_system();

        // Spawn new asteroids periodically
        if self.frame.is_multiple_of(120) {
            self.spawn_asteroid();
        }
    }

    fn movement_system(&mut self, dt: f32) {
        // Query all entities with position and velocity
        for (pos, vel) in self.world.query::<(&mut Position, &Velocity)>() {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;

            // Wrap around screen boundaries
            const WORLD_SIZE: f32 = 500.0;
            if pos.x > WORLD_SIZE {
                pos.x = -WORLD_SIZE;
            }
            if pos.x < -WORLD_SIZE {
                pos.x = WORLD_SIZE;
            }
            if pos.y > WORLD_SIZE {
                pos.y = -WORLD_SIZE;
            }
            if pos.y < -WORLD_SIZE {
                pos.y = WORLD_SIZE;
            }
        }
    }

    fn collision_system(&mut self) {
        // Collect entities for collision detection
        let mut entities_to_damage = Vec::new();

        // Get all bullets
        let bullets: Vec<_> = self
            .world
            .query::<(EntityId, &Position, &Radius, &Bullet)>()
            .map(|(id, pos, radius, _)| (id, *pos, *radius))
            .collect();

        // Get all asteroids
        let asteroids: Vec<_> = self
            .world
            .query::<(EntityId, &Position, &Radius, &Asteroid)>()
            .map(|(id, pos, radius, _)| (id, *pos, *radius))
            .collect();

        // Check bullet-asteroid collisions
        for (bullet_id, bullet_pos, bullet_radius) in &bullets {
            for (asteroid_id, asteroid_pos, asteroid_radius) in &asteroids {
                let dx = bullet_pos.x - asteroid_pos.x;
                let dy = bullet_pos.y - asteroid_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();
                let collision_distance = bullet_radius.value + asteroid_radius.value;

                if distance < collision_distance {
                    // Record collision
                    entities_to_damage.push((*bullet_id, 999)); // Destroy bullet
                    entities_to_damage.push((*asteroid_id, 1)); // Damage asteroid
                }
            }
        }

        // Apply damage
        for (entity_id, damage_amount) in entities_to_damage {
            if let Some(health) = self.world.get_mut::<Health>(entity_id) {
                health.current -= damage_amount;
            }
        }
    }

    fn cleanup_system(&mut self) {
        // Collect dead entities
        let dead_entities: Vec<_> = self
            .world
            .query::<(EntityId, &Health)>()
            .filter(|(_, health)| health.current <= 0)
            .map(|(id, _)| id)
            .collect();

        // Despawn dead entities
        for entity_id in dead_entities {
            // Check if it was an asteroid for scoring
            if self.world.has::<Asteroid>(entity_id) {
                self.score += 10;
            }
            self.world.despawn(entity_id);
        }
    }

    fn spawn_asteroid(&mut self) {
        let angle = (self.frame as f32 * 0.1) % (2.0 * PI);
        let distance = 300.0;
        self.world
            .spawn()
            .with(Asteroid)
            .with(Position {
                x: angle.cos() * distance,
                y: angle.sin() * distance,
            })
            .with(Velocity {
                x: angle.cos() * -30.0,
                y: angle.sin() * -30.0,
            })
            .with(Health { current: 3, max: 3 })
            .with(Radius { value: 20.0 })
            .id();
    }

    fn fire_bullet(&mut self) {
        // Get player position and rotation
        if let Some((_player_id, player_pos, player_rot)) = self
            .world
            .query::<(EntityId, &Position, &Rotation, &Player)>()
            .map(|(id, pos, rot, _)| (id, *pos, *rot))
            .next()
        {
            let bullet_speed = 200.0;
            self.world
                .spawn()
                .with(Bullet)
                .with(Position {
                    x: player_pos.x + player_rot.angle.cos() * 15.0,
                    y: player_pos.y + player_rot.angle.sin() * 15.0,
                })
                .with(Velocity {
                    x: player_rot.angle.cos() * bullet_speed,
                    y: player_rot.angle.sin() * bullet_speed,
                })
                .with(Health { current: 1, max: 1 })
                .with(Radius { value: 2.0 })
                .with(Damage { amount: 1 })
                .id();
        }
    }

    fn print_stats(&mut self) {
        let player_count = self.world.query::<(&Player,)>().count();
        let asteroid_count = self.world.query::<(&Asteroid,)>().count();
        let bullet_count = self.world.query::<(&Bullet,)>().count();

        println!(
            "Frame: {} | Score: {} | Entities: {} (Player: {}, Asteroids: {}, Bullets: {})",
            self.frame,
            self.score,
            self.world.len(),
            player_count,
            asteroid_count,
            bullet_count
        );
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("=== PECS Simple Game Example ===\n");
    println!("Simulating an asteroids-style game...\n");

    let mut game = Game::new();

    // Simulate 300 frames (10 seconds at 30 FPS)
    let dt = 1.0 / 30.0;
    let total_frames = 300;

    println!("Starting simulation...\n");

    for frame in 0..total_frames {
        game.update(dt);

        // Fire bullets periodically
        if frame % 30 == 0 {
            game.fire_bullet();
        }

        // Print stats every 60 frames (2 seconds)
        if frame % 60 == 0 {
            game.print_stats();
        }
    }

    println!("\n=== Simulation Complete ===");
    game.print_stats();
    println!("\nFinal Score: {}", game.score);

    // Test persistence
    println!("\n=== Testing Persistence ===");
    println!("Note: Persistence requires SerializableComponent implementation");
    println!("This is a known limitation documented in API_GAPS.md");
}

// Made with Bob
