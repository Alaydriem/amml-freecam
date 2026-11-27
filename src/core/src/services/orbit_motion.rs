use std::f64::consts::TAU;

use crate::services::PathCurve;
use crate::structs::{CameraPose, Vec3};

// circles the camera around a fixed centre at a fixed height, aimed at the centre. The only state that changes is a
// phase wrapped on every write, ramps clamped to [0, 1] and a speed eased between the speeds asked for; the pose is
// derived from them each step, so nothing grows with the length of the orbit
pub struct OrbitMotion {
    pitch_limit: f32,
    centre: Vec3,
    radius: f32,
    height: f32,
    // +1 or -1, so the camera first moves toward its own right
    direction: f64,
    // the aim pitch, fixed because the height and the radius are fixed
    aim_pitch: f32,
    // the start look minus the aim at the start, on the shortest arc; it fades out over the ease-in
    look_offset_yaw: f32,
    look_offset_pitch: f32,
    // radians in [0, TAU); the offset from the centre is radius * (sin, cos) on x and z
    phase: f64,
    // [0, 1]; rises over the ease-in and stays at 1
    look_ramp: f32,
    // [0, 1]; rises over the ease-in, falls over the ease-out
    drive_ramp: f32,
    stopping: bool,
    // blocks per second along the circle, eased toward the speed each step asks for
    speed: f32,
}

impl OrbitMotion {
    // seconds over which the orbit speeds up from rest and turns to face the centre, and slows to rest when stopped
    pub const RAMP_SECONDS: f32 = 1.0;
    // horizontal camera-to-centre distance in blocks below which no orbit starts
    pub const MIN_RADIUS: f32 = 1.0;
    // per second; the share of a speed change left after t seconds is exp(-SPEED_EASE_RATE * t)
    pub const SPEED_EASE_RATE: f32 = 3.0;

    pub fn new(pitch_limit: f32) -> Self {
        Self {
            pitch_limit,
            centre: Vec3::default(),
            radius: Self::MIN_RADIUS,
            height: 0.0,
            direction: 1.0,
            aim_pitch: 0.0,
            look_offset_yaw: 0.0,
            look_offset_pitch: 0.0,
            phase: 0.0,
            look_ramp: 0.0,
            drive_ramp: 0.0,
            stopping: false,
            speed: 0.0,
        }
    }

    pub fn set_pitch_limit(&mut self, pitch_limit: f32) {
        self.pitch_limit = pitch_limit;
    }

    // false when the camera is closer than MIN_RADIUS to the centre on the horizontal, or a value is not finite;
    // nothing changes then. `speed` is the speed the orbit runs at once the ease-in is over
    pub fn start(&mut self, camera: CameraPose, centre: Vec3, speed: f32) -> bool {
        let offset = camera.position - centre;
        let radius = (offset.x * offset.x + offset.z * offset.z).sqrt();
        let finite = camera.position.is_finite() && centre.is_finite();
        if !finite || !camera.yaw.is_finite() || !camera.pitch.is_finite() || !radius.is_finite() {
            return false;
        }
        if radius < Self::MIN_RADIUS {
            return false;
        }
        let phase = f64::from(offset.x).atan2(f64::from(offset.z));
        let (sin, cos) = phase.sin_cos();
        let tangent = Vec3::new(cos as f32, 0.0, -sin as f32);
        let right = CameraPose::new(Vec3::default(), camera.yaw, 0.0).forward().cross(Vec3::new(0.0, 1.0, 0.0));
        self.centre = centre;
        self.radius = radius;
        self.height = camera.position.y;
        self.direction = if tangent.dot(right) < 0.0 { -1.0 } else { 1.0 };
        self.aim_pitch = (camera.position.y - centre.y).atan2(radius).to_degrees();
        self.look_offset_yaw = PathCurve::wrap_degrees(camera.yaw - Self::aim_yaw(phase));
        self.look_offset_pitch = camera.pitch - self.aim_pitch;
        self.phase = phase.rem_euclid(TAU);
        self.look_ramp = 0.0;
        self.drive_ramp = 0.0;
        self.stopping = false;
        self.speed = Self::usable(speed);
        true
    }

    // advances `dt` seconds toward `speed` blocks per second along the circle; dt 0 gives the same pose again
    pub fn step(&mut self, dt: f32, speed: f32) -> CameraPose {
        let dt = Self::usable(dt);
        let ramp_step = dt / Self::RAMP_SECONDS;
        self.look_ramp = (self.look_ramp + ramp_step).min(1.0);
        self.drive_ramp =
            if self.stopping { self.drive_ramp - ramp_step } else { self.drive_ramp + ramp_step }.clamp(0.0, 1.0);
        let ease = 1.0 - (-Self::SPEED_EASE_RATE * dt).exp();
        self.speed += (Self::usable(speed) - self.speed) * ease;
        let turn = f64::from(self.speed) / f64::from(self.radius) * f64::from(dt);
        self.phase =
            (self.phase + self.direction * turn * f64::from(Self::smoothstep(self.drive_ramp))).rem_euclid(TAU);
        self.pose()
    }

    // starts the ease-out from the speed the orbit has now; a second call does nothing
    pub fn stop(&mut self) {
        self.stopping = true;
    }

    // true once the ease-out is over
    pub fn is_finished(&self) -> bool {
        self.stopping && self.drive_ramp <= 0.0
    }

    fn pose(&self) -> CameraPose {
        let (sin, cos) = self.phase.sin_cos();
        let position =
            Vec3::new(self.centre.x + sin as f32 * self.radius, self.height, self.centre.z + cos as f32 * self.radius);
        let fade = 1.0 - Self::smoothstep(self.look_ramp);
        let yaw = Self::aim_yaw(self.phase) + fade * self.look_offset_yaw;
        let pitch = (self.aim_pitch + fade * self.look_offset_pitch).clamp(-self.pitch_limit, self.pitch_limit);
        CameraPose::new(position, yaw, pitch)
    }

    // the yaw from the camera at `phase` toward the centre, degrees in (-180, 180]
    fn aim_yaw(phase: f64) -> f32 {
        let (sin, cos) = phase.sin_cos();
        sin.atan2(-cos).to_degrees() as f32
    }

    // 0 for a value that is negative or not finite
    fn usable(value: f32) -> f32 {
        if value.is_finite() { value.max(0.0) } else { 0.0 }
    }

    // 0 at 0, 1 at 1, smoothstep between
    fn smoothstep(ramp: f32) -> f32 {
        let x = ramp.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }
}
