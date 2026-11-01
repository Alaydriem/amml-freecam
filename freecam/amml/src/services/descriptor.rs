// what freecam tells the mod manager about itself: claims, keybinds and the settings schema of config.hcl
pub struct FreecamDescriptor;

impl FreecamDescriptor {
    pub const DESCRIPTOR: &'static str = include_str!("../../descriptor.hcl");
}
