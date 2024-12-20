use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

use crate::{DeserCtx, KeyPathElem};

/// Trait for a type that can be serialized to a KDL node.
pub trait NodeSchema: Sized {
    /// Deserializes a KDL node to a value. Emits a warning and returns `None`
    /// if deserialization fails.
    fn from_kdl_node(node: &KdlNode, ctx: DeserCtx<'_>) -> Option<Self>;
    /// Serializes a value to a KDL node.
    fn to_kdl_node(&self) -> KdlNode;
}

/// Trait for a type that can be serialized to a KDL node with any name.
pub trait NodeContentsSchema: Sized {
    /// Deserializes a KDL node to a value, ignoring the name of the node. Emits
    /// a warning and returns `None` if deserialization fails.
    fn from_kdl_node_contents(node: &KdlNode, ctx: DeserCtx<'_>) -> Option<Self>;
    /// Serializes a value to a KDL node with an arbitrary name.
    fn to_kdl_node_with_name(&self, node_name: &str) -> KdlNode;
}

/// Trait for a type that can be serialized to a sequence of KDL nodes.
pub trait DocSchema: Sized {
    /// Deserializes a sequence of KDL nodes to a value. Emits a warning and
    /// returns `None` if deserialization fails.
    fn from_kdl_doc(doc: &KdlDocument, ctx: DeserCtx<'_>) -> Option<Self>;
    /// Serializes a value to a sequence of KDL nodes.
    fn to_kdl_doc(&self) -> KdlDocument;
}

/// Trait for a type that can be serialized to a KDL value.
pub trait ValueSchema: Sized {
    /// Deserializes a KDL entry (value or key-value pair) to a value, ignoring
    /// the key. Emits a warning and returns `None` if deserialization fails.
    fn from_kdl_entry(entry: &KdlEntry, mut ctx: DeserCtx<'_>) -> Option<Self> {
        let mut ctx = match entry.name() {
            Some(name) => ctx.with(KeyPathElem::Property(name)),
            None => ctx,
        };
        let ret = Self::from_kdl_value(entry.value());
        if ret.is_none() {
            ctx.warn_invalid(*entry.span());
        }
        ret
    }

    /// Deserializes a KDL value to a value. Emits a warning and returns `None`
    /// if deserialization fails.
    ///
    /// Prefer [`ValueSchema::from_kdl_entry()`], which emits a warning if
    /// deserialization fails.
    fn from_kdl_value(value: &KdlValue) -> Option<Self>;

    /// Serializes a value to a KDL value.
    fn to_kdl_value(&self) -> KdlValue;
}

/// Trait for a type that acts a proxy for serializing `T` to a KDL node.
pub trait NodeSchemaProxy<T>: Sized {
    /// Deserializes a KDL node into a value.
    fn proxy_from_kdl_node(node: &KdlNode, ctx: DeserCtx<'_>) -> Option<T>;
    /// Serializes a value to a KDL node.
    fn proxy_to_kdl_node(value: &T) -> KdlNode;
}

/// Trait for a type that acts as a proxy for serializing `T` to a KDL node with
/// any name.
pub trait NodeContentsSchemaProxy<T>: Sized {
    /// Deserializes a KDL node to a value, ignoring the name of the node. Emits
    /// a warning and returns `None` if deserialization fails.
    fn proxy_from_kdl_node_contents(node: &KdlNode, ctx: DeserCtx<'_>) -> Option<T>;
    /// Serializes a value to a KDL node with an arbitrary name.
    fn proxy_to_kdl_node_with_name(value: &T, node_name: &str) -> KdlNode;
}

/// Trait for a type that acts as a proxy for serializing `T` to a sequence of
/// KDL nodes.
pub trait DocSchemaProxy<T>: Sized {
    /// Deserializes a sequence of KDL nodes to a value. Emits a warning and
    /// returns `None` if deserialization fails.
    fn proxy_from_kdl_doc(doc: &KdlDocument, ctx: DeserCtx<'_>) -> Option<T>;
    /// Serializes a value to a sequence of KDL nodes.
    fn proxy_to_kdl_doc(value: &T) -> KdlDocument;
}

/// Trait for a type that acts as a proxy for serializing `T` to a KDL value.
pub trait ValueSchemaProxy<T>: Sized {
    /// Deserializes a KDL entry (value or key-value pair) to a value, ignoring
    /// the key. Emits a warning and returns `None` if deserialization fails.
    fn proxy_from_kdl_entry(entry: &KdlEntry, mut ctx: DeserCtx<'_>) -> Option<T> {
        let mut ctx = match entry.name() {
            Some(name) => ctx.with(KeyPathElem::Property(name)),
            None => ctx,
        };
        let ret = Self::proxy_from_kdl_value(entry.value());
        if ret.is_none() {
            ctx.warn_invalid(*entry.span());
        }
        ret
    }

    /// Deserializes a KDL value to a value. Emits a warning and returns `None`
    /// if deserialization fails.
    ///
    /// Prefer [`ValueSchema::from_kdl_entry()`], which emits a warning if
    /// deserialization fails.
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<T>;

    /// Serializes a value to a KDL value.
    fn proxy_to_kdl_value(value: &T) -> KdlValue;
}
