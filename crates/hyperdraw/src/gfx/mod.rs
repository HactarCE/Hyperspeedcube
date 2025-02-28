//! Rendering logic.

#[macro_use]
mod macros;
mod bindings;
mod cache;
mod draw_params;
mod pipelines;
mod placeholder;
mod puzzle;
mod state;
mod structs;

pub use cache::{CachedTexture, CachedTexture1d, CachedTexture2d};
pub use draw_params::DrawParams;
pub use puzzle::PuzzleRenderer;
pub use state::GraphicsState;
pub use structs::GfxEffectParams;

/// Pads a buffer to `wgpu::COPY_BUFFER_ALIGNMENT`.
fn pad_buffer_to_wgpu_copy_buffer_alignment<T: Default + bytemuck::NoUninit>(buf: &mut Vec<T>) {
    loop {
        let bytes_len = bytemuck::cast_slice::<T, u8>(buf).len();
        if bytes_len > 0 && bytes_len as u64 % wgpu::COPY_BUFFER_ALIGNMENT == 0 {
            break;
        }
        buf.push(T::default());
    }
}
