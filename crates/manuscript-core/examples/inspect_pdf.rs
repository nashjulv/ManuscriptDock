//! Local, read-only parser diagnostic. Never copies the input into the repository.
fn main() {
    let path = std::env::args().nth(1).expect("PDF path");
    if std::env::args().any(|arg| arg == "--summary") {
        let root =
            std::env::temp_dir().join(format!("manuscriptdock-inspect-{}", uuid::Uuid::new_v4()));
        let store = manuscript_core::WorkspaceStore::new(&root);
        let workspace = store
            .create_from_source(std::path::Path::new(&path))
            .expect("import temporary copy");
        let report = store.analyze_structure(&workspace.id).expect("analyze");
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        std::fs::remove_dir_all(root).expect("remove this diagnostic's temporary copy");
        return;
    }
    for item in pdf_inspector::extract_text_with_positions(&path).expect("extract") {
        println!(
            "{} {:.1} {:.1} {:.1} {}",
            item.page, item.x, item.y, item.font_size, item.text
        );
    }
}
