use kdl::{KdlDocument, KdlNode, KdlValue};

use super::*;

impl<T: ValueSchema> ValueSchema for Option<T> {
    fn from_kdl_value(value: &KdlValue) -> Option<Self> {
        if value.is_null() {
            Some(None)
        } else {
            T::from_kdl_value(value).map(Some)
        }
    }

    fn to_kdl_value(&self) -> KdlValue {
        match self {
            Some(value) => value.to_kdl_value(),
            None => KdlValue::Null,
        }
    }
}
impl<T, P: ValueSchemaProxy<T>> ValueSchemaProxy<Option<T>> for P {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<Option<T>> {
        if value.is_null() {
            Some(None)
        } else {
            Self::proxy_from_kdl_value(value).map(Some)
        }
    }
    fn proxy_to_kdl_value(value: &Option<T>) -> KdlValue {
        match value {
            Some(value) => Self::proxy_to_kdl_value(value),
            None => KdlValue::Null,
        }
    }
}

impl<T: ValueSchema> NodeContentsSchema for T {
    fn from_kdl_node_contents(node: &KdlNode, mut ctx: DeserCtx<'_>) -> Option<Self> {
        let mut ret = None;
        let mut index = 0;
        for entry in node.entries() {
            match entry.name() {
                Some(k) => ctx.warn_unknown_property(k.value(), *entry.span()),
                None => {
                    if ret.is_none() {
                        ret = Some(T::from_kdl_entry(
                            entry,
                            ctx.with(KeyPathElem::Argument(index)),
                        )?);
                    } else {
                        ctx.warn_unused_arg(index, *entry.span());
                    }
                    index += 1;
                }
            }
        }
        ret
    }
    fn to_kdl_node_with_name(&self, node_name: &str) -> KdlNode {
        let mut node = KdlNode::new(node_name);
        node.push(self.to_kdl_value());
        node
    }
}

impl ValueSchema for String {
    fn from_kdl_value(value: &KdlValue) -> Option<Self> {
        value.as_string().map(|s| s.to_owned())
    }

    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.clone())
    }
}

impl ValueSchema for bool {
    fn from_kdl_value(value: &KdlValue) -> Option<Self> {
        value.as_bool()
    }

    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Bool(*self)
    }
}

impl ValueSchema for i64 {
    fn from_kdl_value(value: &KdlValue) -> Option<Self> {
        value.as_i64()
    }

    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Base10(*self)
    }
}
impl ValueSchema for u32 {
    fn from_kdl_value(value: &KdlValue) -> Option<Self> {
        value.as_i64()?.try_into().ok()
    }

    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Base10(i64::from(*self))
    }
}

impl<T: NodeSchema> NodeContentsSchema for Vec<T> {
    fn from_kdl_node_contents(node: &KdlNode, mut ctx: DeserCtx<'_>) -> Option<Self> {
        node.children().map(|children| {
            children
                .nodes()
                .iter()
                .enumerate()
                .filter_map(|(i, node)| T::from_kdl_node(node, ctx.with(KeyPathElem::Child(i))))
                .collect()
        })
    }

    fn to_kdl_node_with_name(&self, node_name: &str) -> KdlNode {
        let mut node = KdlNode::new(node_name);
        let mut children = KdlDocument::new();
        *children.nodes_mut() = self.iter().map(|elem| elem.to_kdl_node()).collect();
        node.set_children(children);
        node
    }
}
