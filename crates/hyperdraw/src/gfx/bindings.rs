use itertools::Itertools;

pub(in crate::gfx) const fn buffer(
    group: u32,
    binding: u32,
    ty: wgpu::BufferBindingType,
) -> BindingMetadata {
    BindingMetadata {
        group,
        binding,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        visibility: wgpu::ShaderStages::NONE,
    }
}
pub(in crate::gfx) const fn texture(
    group: u32,
    binding: u32,
    view_dimension: wgpu::TextureViewDimension,
    sample_type: wgpu::TextureSampleType,
) -> BindingMetadata {
    BindingMetadata {
        group,
        binding,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension,
            multisampled: false,
        },
        visibility: wgpu::ShaderStages::NONE,
    }
}
pub(in crate::gfx) const fn sampler(
    group: u32,
    binding: u32,
    ty: wgpu::SamplerBindingType,
) -> BindingMetadata {
    BindingMetadata {
        group,
        binding,
        ty: wgpu::BindingType::Sampler(ty),
        visibility: wgpu::ShaderStages::NONE,
    }
}

pub(in crate::gfx) struct BindingMetadata {
    pub group: u32,
    pub binding: u32,
    pub ty: wgpu::BindingType,
    pub visibility: wgpu::ShaderStages,
}

pub struct BindGroups {
    bind_groups: Vec<BindGroup>,
}

pub struct BindGroup {
    base: wgpu::BindGroup,
    offsets: Vec<wgpu::DynamicOffset>,
}

pub(in crate::gfx) trait BindGroupsTrait<'a>: Sized {
    const BINDINGS: &'static [BindingMetadata];

    fn binding_resources(self) -> Vec<wgpu::BindingResource<'a>>;

    fn bind_groups(
        self,
        device: &wgpu::Device,
        label: &str,
        bind_group_layouts: &[wgpu::BindGroupLayout],
    ) -> BindGroups {
        let max_bind_groups = device.limits().max_bind_groups;
        let mut bind_group_entries = vec![vec![]; max_bind_groups as usize];

        for (metadata, resource) in Self::BINDINGS.iter().zip(self.binding_resources()) {
            bind_group_entries
                .get_mut(metadata.group as usize)
                .expect("bind group index out of range")
                .push((metadata.binding, resource));
        }

        // Remove empty bind groups from the end of the list.
        while bind_group_entries
            .last()
            .is_some_and(|group| group.is_empty())
        {
            bind_group_entries.pop();
        }

        let bind_groups = bind_group_entries
            .into_iter()
            .enumerate()
            .map(|(i, entries)| BindGroup {
                base: device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("{label}_bind_group_{i}")),
                    layout: &bind_group_layouts[i],
                    entries: &entries
                        .into_iter()
                        .map(|(binding, resource)| wgpu::BindGroupEntry { binding, resource })
                        .collect_vec(),
                }),
                offsets: vec![],
            })
            .collect_vec();

        BindGroups { bind_groups }
    }

    fn pipeline_layout(
        device: &wgpu::Device,
        label: &str,
    ) -> (wgpu::PipelineLayout, Vec<wgpu::BindGroupLayout>) {
        let max_bind_groups = device.limits().max_bind_groups;
        let mut bind_group_layout_entries = vec![vec![]; max_bind_groups as usize];
        for binding in Self::BINDINGS {
            bind_group_layout_entries
                .get_mut(binding.group as usize)
                .expect("bind group index out of range")
                .push(wgpu::BindGroupLayoutEntry {
                    binding: binding.binding,
                    visibility: binding.visibility,
                    ty: binding.ty,
                    count: None,
                });
        }

        // Remove empty bind groups from the end of the list.
        while bind_group_layout_entries
            .last()
            .is_some_and(|group| group.is_empty())
        {
            bind_group_layout_entries.pop();
        }

        let bind_group_layouts = bind_group_layout_entries
            .iter()
            .enumerate()
            .map(|(i, entries)| {
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("{label}_pipeline_bind_group_layout_{i}")),
                    entries,
                })
            })
            .collect_vec();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{label}_pipeline_layout")),
            bind_group_layouts: &bind_group_layouts.iter().collect_vec(),
            push_constant_ranges: &[],
        });
        (pipeline_layout, bind_group_layouts)
    }
}

pub(in crate::gfx) trait WgpuPassExt {
    fn set_bind_groups(&mut self, bind_groups: &BindGroups);
}
impl WgpuPassExt for wgpu::RenderPass<'_> {
    fn set_bind_groups(&mut self, bind_groups: &BindGroups) {
        for (i, bind_group) in bind_groups.bind_groups.iter().enumerate() {
            self.set_bind_group(i as u32, &bind_group.base, &bind_group.offsets);
        }
    }
}
impl WgpuPassExt for wgpu::ComputePass<'_> {
    fn set_bind_groups(&mut self, bind_groups: &BindGroups) {
        for (i, bind_group) in bind_groups.bind_groups.iter().enumerate() {
            self.set_bind_group(i as u32, &bind_group.base, &bind_group.offsets);
        }
    }
}

pub(in crate::gfx) trait IntoBindingResource<'a> {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a>;
}
impl<'a> IntoBindingResource<'a> for &'a wgpu::Buffer {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a> {
        self.as_entire_binding()
    }
}
impl<'a> IntoBindingResource<'a> for wgpu::BufferBinding<'a> {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::Buffer(self)
    }
}
impl<'a> IntoBindingResource<'a> for &'a [wgpu::BufferBinding<'a>] {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::BufferArray(self)
    }
}
impl<'a> IntoBindingResource<'a> for &'a wgpu::Sampler {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::Sampler(self)
    }
}
impl<'a> IntoBindingResource<'a> for &'a [&'a wgpu::Sampler] {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::SamplerArray(self)
    }
}
impl<'a> IntoBindingResource<'a> for &'a wgpu::TextureView {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::TextureView(self)
    }
}
impl<'a> IntoBindingResource<'a> for &'a [&'a wgpu::TextureView] {
    fn into_binding_resource(self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::TextureViewArray(self)
    }
}
