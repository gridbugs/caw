use anyhow::anyhow;
use lazy_static::lazy_static;
pub use sdl2::ttf::Font;
use sdl2::{rwops::RWops, ttf::Sdl2TtfContext};

lazy_static! {
    static ref TTF_CONTEXT: Result<Sdl2TtfContext, String> = sdl2::ttf::init();
}

pub fn load_font(pt_size: u16) -> anyhow::Result<Font<'static, 'static>> {
    let font_data = include_bytes!("inconsolata.regular.ttf");
    let rwops = RWops::from_bytes(font_data).map_err(|e| anyhow!("{e}"))?;
    let ttf_context = TTF_CONTEXT.as_ref().map_err(|e| anyhow!("{e}"))?;
    ttf_context
        .load_font_from_rwops(rwops, pt_size)
        .map_err(|e| anyhow!(e))
}
