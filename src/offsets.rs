// Code
pub(crate) const UPDATE: usize = 0xf91770;
pub(crate) const SLEEP: usize = 0x578060;
pub(crate) const DISPLAY_LOGOS: usize = 0xC32240;

// Data
pub(crate) const FPS_LIMIT: usize = 0x64563a; // 60
pub(crate) const FPS_LIMIT_2: usize = 0x11e7ff8; // 60.0f64, related to `PlayerIns`?
pub(crate) const FPS_TIME_STEP_DOUBLE: usize = 0x11e7cf0; // 0.03333333507180214f64
pub(crate) const FPS_TIME_STEP_FLOAT: usize = 0x11e7e90; // 0.033333335f32
pub(crate) const FPS_11E7CD8: usize = 0x11e7cd8; // 30.0f64
pub(crate) const FPS_11E7CE0: usize = 0x11e7ce0; // -30f64
