//! Damped harmonic oscillator spring physics for natural-feeling motion.
//!
//! Springs are velocity-based and stateful (per-frame `step()`), unlike
//! duration-based easing which is stateless and fraction-based. The two
//! approaches are unified under [`AnimCurve`](super::behavior::AnimCurve).

use std::f32::consts::PI;

/// Damped harmonic oscillator parameters for spring animations.
///
/// Uses the second-order system model: converts `response` and `damping`
/// to angular frequency (omega) and damping coefficient for the ODE solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// How quickly the spring responds (lower = faster). Default: 0.55.
    /// Corresponds to the period of oscillation in seconds.
    pub response: f32,
    /// Damping ratio. 1.0 = critically damped (no overshoot). Default: 0.825.
    /// < 1.0 = underdamped (overshoot), > 1.0 = overdamped (slow approach).
    pub damping: f32,
    /// Velocity threshold at which animation is considered complete. Default: 0.001.
    /// When `|velocity| < epsilon` AND `|current - target| < epsilon`, done.
    pub epsilon: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            response: 0.55,
            damping: 0.825,
            epsilon: 0.001,
        }
    }
}

/// Maximum `dt` for a single step (1/30 second = ~33ms).
///
/// If a frame takes longer (e.g., due to stutter), large `dt` values can
/// cause the spring to overshoot wildly or diverge. Clamping ensures
/// stability at the cost of slightly slower animation during frame drops.
const MAX_DT: f32 = 1.0 / 30.0;

impl Spring {
    /// Advance the spring simulation by one time step.
    ///
    /// Given the current value, target, velocity, and delta time,
    /// returns `(new_value, new_velocity, is_done)`.
    ///
    /// Uses semi-implicit Euler integration of the damped harmonic oscillator:
    /// ```text
    /// omega = 2 * PI / response
    /// acceleration = omega^2 * (target - current) - 2 * damping * omega * velocity
    /// velocity' = velocity + acceleration * dt
    /// current' = current + velocity' * dt
    /// is_done = |velocity'| < epsilon && |current' - target| < epsilon
    /// ```
    pub fn step(&self, current: f32, target: f32, velocity: f32, dt: f32) -> (f32, f32, bool) {
        // Clamp dt for stability.
        let dt = dt.min(MAX_DT);

        if dt <= 0.0 {
            return (current, velocity, false);
        }

        let omega = 2.0 * PI / self.response;
        let displacement = target - current;

        // Semi-implicit Euler: update velocity first, then position.
        let acceleration = omega * omega * displacement - 2.0 * self.damping * omega * velocity;
        let new_velocity = velocity + acceleration * dt;
        let new_current = current + new_velocity * dt;

        let is_done =
            new_velocity.abs() < self.epsilon && (new_current - target).abs() < self.epsilon;

        (new_current, new_velocity, is_done)
    }
}

#[cfg(test)]
mod tests;
