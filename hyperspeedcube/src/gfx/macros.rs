macro_rules! include_wgsl {
    ($file_path:literal $(, $var:ident)* $(,)?) => {
        ::wgpu::ShaderModuleDescriptor {
            label: Some($file_path),
            source: ::wgpu::ShaderSource::Wgsl(
                include_str!($file_path)
                    $(.replace(
                        concat!("{{", stringify!($var), "}}"),
                        &$var.to_string(),
                    ))*
                    .into(),
            ),
        }
    };
}

macro_rules! single_type_vertex_buffer {
    ($loc:expr => $fmt:ident) => {
        ::wgpu::VertexBufferLayout {
            array_stride: ::wgpu::VertexFormat::$fmt.size(),
            step_mode: ::wgpu::VertexStepMode::Vertex,
            attributes: &::wgpu::vertex_attr_array![$loc => $fmt],
        }
    };
}

macro_rules! blend_component {
    ($operation:ident(src * $src_factor:ident, dst * $dst_factor:ident)) => {
        ::wgpu::BlendComponent {
            src_factor: ::wgpu::BlendFactor::$src_factor,
            dst_factor: ::wgpu::BlendFactor::$dst_factor,
            operation: ::wgpu::BlendOperation::$operation,
        }
    };
}

macro_rules! bindings_struct {
    ($vis:vis struct $bindings_struct_name:ident<$lt:lifetime> {
        $($binding_name:ident: $binding_type:ty = pub($visibility:ident) $binding_data:expr),* $(,)?
    }) => {
        $vis struct $bindings_struct_name<$lt> {
            $(pub $binding_name: $binding_type),*
        }
        impl<$lt> $crate::gfx::bindings::BindGroupsTrait<$lt> for $bindings_struct_name<$lt> {
            const BINDINGS: &'static [$crate::gfx::bindings::BindingMetadata] = &[$({
                $crate::gfx::bindings::BindingMetadata {
                    visibility: ::wgpu::ShaderStages::$visibility,
                    ..$binding_data
                }
            }),*];

            fn binding_resources(self) -> Vec<::wgpu::BindingResource<$lt>> {
                vec![$($crate::gfx::bindings::IntoBindingResource::into_binding_resource(self.$binding_name)),*]
            }
        }
    };
}

macro_rules! pipeline {
    ($vis:vis struct $pipeline_name:ident {
        type = $base_pipeline_type:ty;

        // Bindings
        struct $bindings_struct_name:ident<$bindings_lt:lifetime> {
            $($bindings_tok:tt)*
        }

        // Parameters to pipeline descriptor
        $(
            struct $pipeline_params_struct_name:ident
                $(<$($pipeline_params_lt:lifetime),* $(,)?>)?
            {
                $($pipeline_param:ident: $pipeline_param_type:ty),* $(,)?
            }
        )?
        // Pipeline descriptor
        let pipeline_descriptor = $pipeline_descriptor_expr:expr;
    }) => {
        $vis struct $pipeline_name {
            pub pipeline: $base_pipeline_type,
            pub device: ::std::sync::Arc<::wgpu::Device>,
            pub label: String,
            pub bind_group_layouts: Vec<::wgpu::BindGroupLayout>,
        }

        bindings_struct!($vis struct $bindings_struct_name<$bindings_lt> {
            $($bindings_tok)*
        });

        $(
            $vis struct $pipeline_params_struct_name $(<$($pipeline_params_lt),*>)? {
                $(pub $pipeline_param: $pipeline_param_type),*
            }
        )?

        impl $pipeline_name {
            pub fn new<$bindings_lt, $( $($($pipeline_params_lt),*)? )?> (
                device: &::std::sync::Arc<::wgpu::Device>,
                shader_module: &::wgpu::ShaderModule,
                $( params: $pipeline_params_struct_name $(<$($pipeline_params_lt),*>)?, )?
            ) -> Self {
                // Unpack parameters
                $( let $pipeline_params_struct_name { $($pipeline_param),* } = params; )?
                // Build descriptor
                let desc = $pipeline_descriptor_expr;

                let label = desc.label.to_string();
                let (pipeline_layout, bind_group_layouts) =
                    <$bindings_struct_name<$bindings_lt> as $crate::gfx::bindings::BindGroupsTrait>::pipeline_layout(
                        &device, &label
                    );

                Self {
                    pipeline: desc.create_pipeline(
                        &device,
                        shader_module,
                        &pipeline_layout
                    ),
                    device: ::std::sync::Arc::clone(device),
                    label,
                    bind_group_layouts,
                }
            }

            pub fn bind_groups<$bindings_lt>(
                &self,
                bindings: $bindings_struct_name<$bindings_lt>,
            ) -> $crate::gfx::BindGroups {
                $crate::gfx::bindings::BindGroupsTrait::bind_groups(
                    bindings,
                    &self.device,
                    &self.label,
                    &self.bind_group_layouts,
                )
            }
        }
    };
}
