use avian3d::prelude::*;
use bevy::prelude::*;

/// Result of a bullet trajectory simulation
#[derive(Debug, Clone)]
pub struct BulletTrajectoryResult {
    /// The entity that was hit, if any
    pub hit_entity: Option<Entity>,
    /// The point where the bullet hit or the final position
    pub hit_point: Vec3,
    /// The distance traveled by the bullet
    pub distance: f32,
    /// The time it took for the bullet to reach the hit point
    pub time_of_flight: f32,
    /// The path the bullet took (sampled points)
    pub trajectory_points: Vec<Vec3>,
    /// The velocity at impact
    pub impact_velocity: Vec3,
}

/// Configuration for bullet physics simulation
#[derive(Debug, Clone)]
pub struct BulletPhysicsConfig {
    /// Gravity acceleration (default: -9.81 m/s² on Y axis)
    pub gravity: Vec3,
    /// Air resistance coefficient (default: 0.47 for a sphere)
    pub drag_coefficient: f32,
    /// Cross-sectional area of the bullet in m² (default: 0.00002 for a 5mm bullet)
    pub cross_section_area: f32,
    /// Air density in kg/m³ (default: 1.225 at sea level)
    pub air_density: f32,
    /// Maximum simulation time in seconds (default: 10.0)
    pub max_time: f32,
    /// Time step for simulation in seconds (default: 0.001)
    pub time_step: f32,
    /// Maximum distance to simulate (default: 1000.0)
    pub max_distance: f32,
}

impl Default for BulletPhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            drag_coefficient: 0.47,
            cross_section_area: 0.00002, // ~5mm bullet
            air_density: 1.225,
            max_time: 30.0,
            time_step: 0.001,
            max_distance: 10000.0,
        }
    }
}

#[allow(dead_code)]
impl BulletPhysicsConfig {
    /// 9mm Parabellum configuration
    pub fn caliber_9mm() -> Self {
        Self {
            drag_coefficient: 0.295,       // Typical for FMJ bullets
            cross_section_area: 0.0000636, // 9mm diameter
            ..Default::default()
        }
    }

    /// 5.56x45mm NATO / .223 Remington configuration
    pub fn caliber_556() -> Self {
        Self {
            drag_coefficient: 0.275,       // Typical for rifle bullets
            cross_section_area: 0.0000243, // 5.56mm diameter
            ..Default::default()
        }
    }

    /// 7.62x51mm NATO / .308 Winchester configuration
    pub fn caliber_762() -> Self {
        Self {
            drag_coefficient: 0.290,
            cross_section_area: 0.0000456, // 7.62mm diameter
            ..Default::default()
        }
    }

    /// .50 BMG (12.7x99mm) configuration
    pub fn caliber_50bmg() -> Self {
        Self {
            drag_coefficient: 0.320,
            cross_section_area: 0.0001267, // 12.7mm diameter
            ..Default::default()
        }
    }
}

/// Trait extension for SpatialQuery to simulate bullet trajectories
pub trait BulletTrajectory {
    /// Simulates a bullet trajectory with physics until it hits something
    ///
    /// # Parameters
    /// - `start_position`: Initial position of the bullet
    /// - `initial_velocity`: Initial velocity vector of the bullet (m/s)
    /// - `mass`: Mass of the bullet in kg
    /// - `config`: Optional physics configuration (uses defaults if None)
    /// - `filter`: Spatial query filter for collision detection
    ///
    /// # Returns
    /// A `BulletTrajectoryResult` containing hit information and trajectory data
    fn simulate_bullet_trajectory(
        &self,
        start_position: Vec3,
        initial_velocity: Vec3,
        mass: f32,
        config: Option<BulletPhysicsConfig>,
        filter: &SpatialQueryFilter,
    ) -> BulletTrajectoryResult;

    /// Simulates a simple ballistic trajectory without air resistance
    ///
    /// # Parameters
    /// - `start_position`: Initial position of the bullet
    /// - `initial_velocity`: Initial velocity vector of the bullet (m/s)
    /// - `gravity`: Gravity acceleration vector (default: -9.81 on Y)
    /// - `filter`: Spatial query filter for collision detection
    ///
    /// # Returns
    /// A `BulletTrajectoryResult` containing hit information and trajectory data
    fn simulate_simple_trajectory(
        &self,
        start_position: Vec3,
        initial_velocity: Vec3,
        gravity: Option<Vec3>,
        filter: &SpatialQueryFilter,
    ) -> BulletTrajectoryResult;
}

impl BulletTrajectory for SpatialQuery<'_, '_> {
    fn simulate_bullet_trajectory(
        &self,
        start_position: Vec3,
        initial_velocity: Vec3,
        mass: f32,
        config: Option<BulletPhysicsConfig>,
        filter: &SpatialQueryFilter,
    ) -> BulletTrajectoryResult {
        let config = config.unwrap_or_default();

        let mut position = start_position;
        let mut velocity = initial_velocity;
        let mut trajectory_points = vec![position];
        let mut time = 0.0;
        let mut total_distance = 0.0;

        // Pre-calculate drag constant for efficiency
        let drag_constant =
            0.5 * config.air_density * config.drag_coefficient * config.cross_section_area;

        while time < config.max_time && total_distance < config.max_distance {
            // Calculate drag force: F_drag = 0.5 * ρ * C_d * A * v²
            let velocity_magnitude = velocity.length();
            let drag_force = if velocity_magnitude > 0.0 {
                -drag_constant * velocity_magnitude * velocity / mass
            } else {
                Vec3::ZERO
            };

            // Total acceleration = gravity + drag/mass
            let acceleration = config.gravity + drag_force;

            // Update velocity and position using Euler integration
            let new_velocity = velocity + acceleration * config.time_step;
            let new_position = position + velocity * config.time_step;

            // Check for collision along the path segment
            let segment_vector = new_position - position;
            let segment_distance = segment_vector.length();

            // Convert Vec3 to Dir3 for the cast_ray function
            if let Ok(segment_direction) = Dir3::try_from(segment_vector)
                && let Some(hit) =
                    self.cast_ray(position, segment_direction, segment_distance, true, filter)
            {
                // We hit something!
                let hit_point = position + segment_direction * hit.distance;
                trajectory_points.push(hit_point);

                return BulletTrajectoryResult {
                    hit_entity: Some(hit.entity),
                    hit_point,
                    distance: total_distance + hit.distance,
                    time_of_flight: time + (hit.distance / velocity_magnitude) * config.time_step,
                    trajectory_points,
                    impact_velocity: velocity,
                };
            }

            // Update for next iteration
            position = new_position;
            velocity = new_velocity;
            time += config.time_step;
            total_distance += segment_distance;

            trajectory_points.push(position);
        }

        // No hit found within simulation limits
        BulletTrajectoryResult {
            hit_entity: None,
            hit_point: position,
            distance: total_distance,
            time_of_flight: time,
            trajectory_points,
            impact_velocity: velocity,
        }
    }

    fn simulate_simple_trajectory(
        &self,
        start_position: Vec3,
        initial_velocity: Vec3,
        gravity: Option<Vec3>,
        filter: &SpatialQueryFilter,
    ) -> BulletTrajectoryResult {
        let gravity = gravity.unwrap_or(Vec3::new(0.0, -9.81, 0.0));
        let time_step = 0.01; // 10ms steps for simple simulation
        let max_time = 30.0;
        let max_distance = 10000.0;

        let mut position = start_position;
        let mut velocity = initial_velocity;
        let mut trajectory_points = vec![position];
        let mut time = 0.0;
        let mut total_distance = 0.0;

        while time < max_time && total_distance < max_distance {
            // Simple physics: v = v0 + at, s = s0 + vt
            let new_velocity = velocity + gravity * time_step;
            let new_position = position + velocity * time_step;

            // Check for collision
            let segment_vector = new_position - position;
            let segment_distance = segment_vector.length();

            if segment_distance > 0.0 {
                // Convert Vec3 to Dir3 for the cast_ray function
                if let Ok(segment_direction) = Dir3::try_from(segment_vector)
                    && let Some(hit) =
                        self.cast_ray(position, segment_direction, segment_distance, true, filter)
                {
                    let hit_point = position + segment_direction * hit.distance;
                    trajectory_points.push(hit_point);

                    return BulletTrajectoryResult {
                        hit_entity: Some(hit.entity),
                        hit_point,
                        distance: total_distance + hit.distance,
                        time_of_flight: time + (hit.distance / velocity.length()) * time_step,
                        trajectory_points,
                        impact_velocity: velocity,
                    };
                }
            }

            position = new_position;
            velocity = new_velocity;
            time += time_step;
            total_distance += segment_distance;

            // Store trajectory points periodically
            if (trajectory_points.len() as f32 * 0.1) <= time {
                trajectory_points.push(position);
            }
        }

        BulletTrajectoryResult {
            hit_entity: None,
            hit_point: position,
            distance: total_distance,
            time_of_flight: time,
            trajectory_points,
            impact_velocity: velocity,
        }
    }
}
