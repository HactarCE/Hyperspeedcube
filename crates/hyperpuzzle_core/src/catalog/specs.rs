use super::*;

/// Possible ID redirect.
#[derive(Debug, Clone)]
pub enum Redirectable<T> {
    /// Thing directly generated.
    Direct(T),
    /// Redirect to a different ID.
    Redirect(String),
}

/// Context when building an object in the catalog.
#[derive(Clone)]
pub struct BuildCtx {
    /// Logging output.
    pub logger: Logger,
    /// Progress output.
    pub progress: Arc<Mutex<Progress>>,
}
impl BuildCtx {
    pub(super) fn new(logger: &Logger, progress: &Arc<Mutex<Progress>>) -> Self {
        Self {
            logger: logger.clone(),
            progress: Arc::clone(progress),
        }
    }
}

/// Output of a `build` or `generate` function.
pub type BuildResult<T, E = eyre::Report> = Result<Redirectable<Arc<T>>, E>;

/// Puzzle type specification.
pub struct PuzzleSpec {
    /// Basic metadata.
    pub meta: Arc<PuzzleListMetadata>,
    /// Function to build the puzzle.
    ///
    /// **This may be expensive. Do not call it from the UI thread.**
    pub build: Box<dyn Send + Sync + Fn(BuildCtx) -> BuildResult<Puzzle>>,
}
impl fmt::Debug for PuzzleSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleSpec")
            .field("meta", &self.meta)
            .finish()
    }
}

/// Puzzle type specification generator.
pub struct PuzzleSpecGenerator {
    /// Basic metadata.
    pub meta: Arc<PuzzleListMetadata>,
    /// Parameter types, ranges, and defaults.
    pub params: Vec<GeneratorParam>,
    /// Example puzzles, indexed by ID.
    pub examples: HashMap<String, Arc<PuzzleSpec>>,
    /// Function to generate the puzzle type specification.
    ///
    /// **This may be expensive. Do not call it from UI thread.**
    pub generate: Box<dyn Send + Sync + Fn(BuildCtx, Vec<String>) -> BuildResult<PuzzleSpec>>,
}
impl fmt::Debug for PuzzleSpecGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleSpec")
            .field("meta", &self.meta)
            .field("params", &self.params)
            .finish()
    }
}

/// Twist system specification.
pub struct TwistSystemSpec {
    /// Twist system ID.
    pub id: String,
    /// Twist system name.
    pub name: String,
    /// Function to build the twist system.
    ///
    /// **This may be expensive. Do not call it from the UI thread.**
    pub build: Box<dyn Send + Sync + Fn(BuildCtx) -> BuildResult<TwistSystem>>,
}
impl HasId for TwistSystemSpec {
    fn id(&self) -> &str {
        &self.id
    }
}

/// Color system generator.
pub type ColorSystemGenerator = Generator<ColorSystem>;
/// Twist system generator.
pub type TwistSystemSpecGenerator = Generator<TwistSystemSpec>;

/// Object specification generator.
pub struct Generator<T> {
    /// Internal ID.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// Parameter types, ranges, and defaults.
    pub params: Vec<GeneratorParam>,
    /// Function to generate the object specification.
    ///
    /// **This may be expensive. Do not call it from UI thread.**
    pub generate: Box<dyn Send + Sync + Fn(BuildCtx, Vec<String>) -> BuildResult<T>>,
}
impl fmt::Debug for ColorSystemGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorSystemGenerator")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("params", &self.params)
            .finish()
    }
}

/// Common metadata about a puzzle or puzzle generator.
///
/// This is a particularly useful abstraction for displaying the puzzle list.
#[derive(Serialize, Debug, Clone)]
pub struct PuzzleListMetadata {
    /// Internal ID.
    pub id: String,
    /// Semantic version, in the form `[major, minor, patch]`.
    ///
    /// - Major version changes indicate that log files may be incompatible.
    /// - Minor version changes indicate that scrambles may be incompatible.
    /// - Patch versions indicate any other changes, including user-facing
    ///   changes.
    /// - Major version `0` allows any breaking changes.
    pub version: Version,
    /// Human-friendly name.
    pub name: String,
    /// Human-friendly aliases.
    pub aliases: Vec<String>,
    /// Set of tags and associated values.
    pub tags: TagSet,
}

/// Compare by puzzle ID.
impl PartialEq for PuzzleListMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
/// Compare by puzzle ID.
impl Eq for PuzzleListMetadata {}
/// Compare by puzzle ID.
impl PartialOrd for PuzzleListMetadata {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
/// Compare by puzzle ID.
impl Ord for PuzzleListMetadata {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        crate::compare_ids(&self.id, &other.id)
    }
}
