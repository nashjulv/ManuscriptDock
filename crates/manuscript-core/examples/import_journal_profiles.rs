use manuscript_core::JournalDirectoryStore;
use std::{env, path::PathBuf, process};

fn main() {
    let mut arguments = env::args_os().skip(1);
    let Some(root) = arguments.next() else {
        eprintln!("usage: import_journal_profiles <workspace-root> <profiles.jsonl>");
        process::exit(2);
    };
    let Some(profile_path) = arguments.next() else {
        eprintln!("a profile JSONL file is required");
        process::exit(2);
    };
    if arguments.next().is_some() {
        eprintln!("only one profile JSONL file can be imported at a time");
        process::exit(2);
    }
    let store = JournalDirectoryStore::new(PathBuf::from(root).join("journal-directory"));
    match store.import_profile_catalog(&PathBuf::from(profile_path)) {
        Ok(result) => println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("profile result is serializable")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}
