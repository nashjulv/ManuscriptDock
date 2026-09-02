use manuscript_core::JournalDirectoryStore;
use std::{env, path::PathBuf, process};

fn main() {
    let mut arguments = env::args_os().skip(1);
    let Some(root) = arguments.next() else {
        eprintln!("usage: import_journal_directory <workspace-root> <workbook.xlsx> [...]");
        process::exit(2);
    };
    let workbooks = arguments.map(PathBuf::from).collect::<Vec<_>>();
    if workbooks.is_empty() {
        eprintln!("at least one workbook is required");
        process::exit(2);
    }
    let store = JournalDirectoryStore::new(PathBuf::from(root).join("journal-directory"));
    match store.import_workbooks(&workbooks) {
        Ok(result) => println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("import result is serializable")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}
