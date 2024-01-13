//! Rendering logic.

#[macro_use]
mod macros;
mod bindings;
mod cache;
mod pipelines;
mod puzzle;
mod state;
mod structs;

use bindings::BindGroups;
use cache::{CachedTexture1d, CachedTexture2d};
pub(crate) use puzzle::{PuzzleRenderResources, PuzzleRenderer, RenderEngine, ViewParams};
pub(crate) use state::GraphicsState;

/// Pads a buffer to `wgpu::COPY_BUFFER_ALIGNMENT`.
pub(super) fn pad_buffer_to_wgpu_copy_buffer_alignment<T: Default + bytemuck::NoUninit>(
    buf: &mut Vec<T>,
) {
    loop {
        let bytes_len = bytemuck::cast_slice::<T, u8>(buf).len();
        if bytes_len > 0 && bytes_len as u64 % wgpu::COPY_BUFFER_ALIGNMENT == 0 {
            break;
        }
        buf.push(T::default());
    }
}
