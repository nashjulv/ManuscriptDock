pub mod ai_assistance;
mod compiler;
mod documents;
mod domain;
mod journals;
pub mod material_drafts;
pub mod material_review;
pub mod package_workspace;
mod preparation;
mod projects;

pub use compiler::{build_package, export_package, ExportDestination};
pub use documents::{
    inspect_docx, inspect_pdf, DocumentFeatureProfile, DocxInspection, PdfInspection,
};
pub use domain::*;
pub use journals::{catalog, find_journals, recommend, rules_for, rules_hash_for};
pub use preparation::prepare;
pub use projects::ProjectStore;

pub const MAX_MANUSCRIPT_SIZE_BYTES: u64 = 250 * 1024 * 1024;
pub const MAX_MATERIAL_FILES: usize = 200;
pub const MAX_MATERIAL_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
