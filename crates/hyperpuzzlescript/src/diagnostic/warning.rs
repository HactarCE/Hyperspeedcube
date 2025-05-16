use crate::FileId;

use super::ReportBuilder;

/// Warning type for the language.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Warning {}

impl Warning {
    /// Returns the error as a string with ANSI escape codes.
    pub fn to_string(&self, files: impl ariadne::Cache<FileId>) -> String {
        self.report().to_string_with_ansi_escapes(files)
    }

    fn report(&self) -> ReportBuilder {
        match self {
            _ => todo!(),
        }
    }
}
