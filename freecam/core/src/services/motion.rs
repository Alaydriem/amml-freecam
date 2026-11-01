use crate::config::MotionConfig;
use crate::structs::{CameraPose, HeldKeys, Vec3};

pub struct CameraMotion {
    config: MotionConfig,
    pose: CameraPose,
    velocity: Vec3,
    yaw_goal: f32,
    pitch_goal: f32,
    speed: f32,
    smooth: bool,
}

impl CameraMotion {
    pub fn new(config: MotionConfig) -> Self {
        let speed = config.start_speed;
        let smooth = config.smooth_at_start;
        Self {
            config,
            pose: CameraPose::default(),
            velocity: Vec3::default(),
            yaw_goal: 0.0,
            pitch_goal: 0.0,
            speed,
            smooth,
        }
    }

    // a new config while freecam runs: the pose stays, a changed start speed or smooth flag takes over, and the speed
    // moves into the new range; an unchanged one leaves a wheel or toggle change of this session alone
    pub fn set_config(&mut self, config: MotionConfig) {
        if config.start_speed != self.config.start_speed {
            self.speed = config.start_speed;
        }
        if config.smooth_at_start != self.config.smooth_at_start {
            self.smooth = config.smooth_at_start;
        }
        self.speed = self.speed.clamp(config.min_speed, config.max_speed);
        self.config = config;
    }

    // places the camera at start and keeps the current speed
    pub fn reset(&mut self, start: CameraPose) {
        self.pose = start;
        self.velocity = Vec3::default();
        self.yaw_goal = start.yaw;
        self.pitch_goal = start.pitch;
    }

    // takes raw mouse counts
    pub fn add_look(&mut self, dx: f32, dy: f32) {
        let limit = self.config.pitch_limit;
        self.yaw_goal += dx * self.config.sensitivity;
        self.pitch_goal = (self.pitch_goal + dy * self.config.sensitivity).clamp(-limit, limit);
    }

    pub fn scroll(&mut self, notches: i32) {
        let scaled = self.speed * self.config.scroll_factor.powi(notches);
        self.speed = scaled.clamp(self.config.min_speed, self.config.max_speed);
    }

    pub fn toggle_smooth(&mut self) -> bool {
        self.smooth = !self.smooth;
        self.smooth
    }

    pub fn is_smooth(&self) -> bool {
        self.smooth
    }

    pub fn hold(&mut self) {
        self.velocity = Vec3::default();
    }

    pub fn step(&mut self, dt: f32, held: HeldKeys) -> CameraPose {
        let dt = dt.clamp(0.0, self.config.max_frame_dt);
        let look_k = self.blend(self.config.look_ease_rate, dt);
        self.pose.yaw += (self.yaw_goal - self.pose.yaw) * look_k;
        self.pose.pitch += (self.pitch_goal - self.pose.pitch) * look_k;

        let multiplier = if held.fast { self.config.fast_multiplier } else { 1.0 };
        let target = self.wish(held) * (self.speed * multiplier);
        let move_k = self.blend(self.config.move_ease_rate, dt);
        self.velocity = self.velocity + (target - self.velocity) * move_k;
        self.pose.position = self.pose.position + self.velocity * dt;
        self.pose
    }

    pub fn pose(&self) -> CameraPose {
        self.pose
    }

    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    // fraction of the remaining distance covered this frame
    fn blend(&self, rate: f32, dt: f32) -> f32 {
        if self.smooth { 1.0 - (-rate * dt).exp() } else { 1.0 }
    }

    // unit direction the held keys ask for, in world space
    fn wish(&self, held: HeldKeys) -> Vec3 {
        let level = CameraPose::new(Vec3::default(), self.pose.yaw, 0.0).forward();
        let side = level.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
        let forward_axis = Self::axis(held.forward, held.back);
        let side_axis = Self::axis(held.right, held.left);
        let flat = (level * forward_axis + side * side_axis).normalized();
        flat + Vec3::new(0.0, Self::axis(held.up, held.down), 0.0)
    }

    fn axis(positive: bool, negative: bool) -> f32 {
        f32::from(u8::from(positive)) - f32::from(u8::from(negative))
    }
}
