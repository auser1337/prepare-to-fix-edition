use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) fps_cap: u8, // 0-255, of course
}
