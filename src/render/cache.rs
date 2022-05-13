use super::GraphicsState;

pub(crate) struct CachedBuffer {
    f: fn(&GraphicsState, usize) -> wgpu::Buffer,
    len: Option<usize>,
    buffer: Option<wgpu::Buffer>,
}
impl CachedBuffer {
    pub(super) fn new(f: fn(&GraphicsState, usize) -> wgpu::Buffer) -> Self {
        Self {
            f,
            len: None,
            buffer: None,
        }
    }

    pub(super) fn at_min_len(&mut self, gfx: &GraphicsState, min_len: usize) -> &mut wgpu::Buffer {
        // Invalidate the buffer if it is too small.
        if let Some(len) = self.len {
            if len < min_len {
                self.buffer = None;
            }
        }

        self.buffer.get_or_insert_with(|| {
            self.len = Some(min_len);
            (self.f)(gfx, min_len)
        })
    }

    pub(super) fn slice(
        &mut self,
        gfx: &GraphicsState,
        len: usize,
    ) -> (&wgpu::Buffer, wgpu::BufferSlice) {
        let b = self.at_min_len(gfx, len);
        (b, b.slice(0..len as u64))
    }
}
