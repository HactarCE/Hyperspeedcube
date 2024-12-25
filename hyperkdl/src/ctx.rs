use std::fmt;

use kdl::KdlIdentifier;
use miette::SourceSpan;

/// Error or warning when deserializing a KDL structure.
#[derive(Debug, Clone)]
pub struct Warning {
    /// Span in the KDL file.
    pub span: SourceSpan,
    /// Warning message.
    pub msg: String,
}
impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { span, msg } = self;
        write!(f, "{span:?}: {msg}")
    }
}

/// KDL deserialization context.
#[derive(Debug)]
pub struct DeserCtx<'a> {
    path: Option<KeyPath<'a>>,
    warnings: &'a mut Vec<Warning>,
}
impl<'a> DeserCtx<'a> {
    /// Creates a new deserialization context.
    pub fn new(warnings: &'a mut Vec<Warning>) -> Self {
        let path = None;
        DeserCtx { path, warnings }
    }

    /// Adds an element to the key path.
    pub fn with<'b>(&'b mut self, key: KeyPathElem<'b>) -> DeserCtx<'b> {
        DeserCtx {
            path: Some(KeyPath {
                parent: self.path.as_ref(),
                key,
            }),
            warnings: self.warnings,
        }
    }

    fn path_string(&self) -> String {
        match self.path {
            Some(path) => path.to_string(),
            None => ".".to_string(),
        }
    }

    /// Adds a warning: "expected node name {expected:?}"
    #[doc(hidden)]
    pub fn warn_wrong_node_name(&mut self, expected: &str, got: &str, span: SourceSpan) {
        let msg = format!("expected node name {expected:?}; got {got:?}");
        self.warn(span, msg);
    }
    /// Adds a warning: "invalid value"
    #[doc(hidden)]
    pub fn warn_invalid(&mut self, span: SourceSpan) {
        self.warn(span, "invalid value");
    }
    /// Adds a warning: "unknown property {k:?}"
    #[doc(hidden)]
    pub fn warn_unknown_property(&mut self, k: &str, span: SourceSpan) {
        self.warn(span, format!("unknown property {k:?}"));
    }
    /// Adds a warning: "unknown child {k:?}"
    #[doc(hidden)]
    pub fn warn_unknown_child(&mut self, k: &str, span: SourceSpan) {
        self.warn(span, format!("unknown child {k:?}"));
    }
    /// Adds a warning: "unknown node name {k:?}"
    #[doc(hidden)]
    pub fn warn_unknown_node_name(&mut self, k: &str, span: SourceSpan) {
        self.warn(span, format!("unknown node name {k:?}"));
    }
    /// Adds a warning: "unused argument #{index}"
    #[doc(hidden)]
    pub fn warn_unused_arg(&mut self, index: usize, span: SourceSpan) {
        self.warn(span, format!("unused argument #{index}"));
    }
    /// Adds a warning: "unused children"
    #[doc(hidden)]
    pub fn warn_unused_children(&mut self, span: SourceSpan) {
        self.warn(span, "unused children");
    }
    /// Adds a warning: "missing {thing_missing:?}"
    #[doc(hidden)]
    pub fn warn_missing(&mut self, thing_missing: &str, span: SourceSpan) {
        self.warn(span, format!("missing {thing_missing:?}"));
    }

    fn warn(&mut self, span: SourceSpan, msg: impl fmt::Display) {
        let msg = format!("{msg} at {}", self.path_string());
        self.warnings.push(Warning { span, msg });
    }
}

/// Semantic location in a KDL file, used when generating warning messages.
#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub struct KeyPath<'a> {
    parent: Option<&'a KeyPath<'a>>,
    key: KeyPathElem<'a>,
}
impl fmt::Display for KeyPath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(parent) = self.parent {
            write!(f, "{parent}/")?;
        }
        write!(f, "{}", self.key)
    }
}

/// Element of a semantic location in a KDL file.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum KeyPathElem<'a> {
    Argument(usize),
    Child(usize),
    Node(&'a KdlIdentifier),
    Property(&'a KdlIdentifier),
}
impl fmt::Display for KeyPathElem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyPathElem::Argument(i) => write!(f, "arg#{i}"),
            KeyPathElem::Child(i) => write!(f, "child#{i}"),
            KeyPathElem::Node(name) => write!(f, "{name}"),
            KeyPathElem::Property(name) => write!(f, "{name}"),
        }
    }
}
