use rusqlite::{Connection, Result};
use std::fs;

#[test]
fn test_git_vfs_intercepts_db_files() -> Result<()> {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("my_versioned_file.db");

    // Standard open should use the globally registered git-sqlite-vfs
    // because we patched rusqlite's open_with_flags to register it as default.
    let conn = Connection::open(&db_path)?;
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", [])?;
    
    // Insert enough data to ensure multiple pages are flushed to disk
    for i in 0..100 {
        conn.execute(
            "INSERT INTO users (name) VALUES (?1)",
            [format!("User {}", i)],
        )?;
    }
    
    // Close connection to ensure everything is flushed
    drop(conn);

    // Verify that the 'db_path' is a DIRECTORY (as gitvfs does), not a flat file.
    let metadata = fs::metadata(&db_path).expect("Database path should exist");
    assert!(metadata.is_dir(), "git-sqlite-vfs should create a directory, not a flat file!");
    
    // Check if the 'pages' directory exists inside
    let pages_dir = db_path.join("pages");
    let pages_metadata = fs::metadata(&pages_dir).expect("pages directory should exist");
    assert!(pages_metadata.is_dir(), "pages directory should exist inside the vfs directory");

    // Verify files exist inside pages directory
    let mut page_files_count = 0;
    for entry in fs::read_dir(&pages_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.metadata().unwrap().is_file() {
            page_files_count += 1;
        }
    }
    assert!(page_files_count > 0, "There should be sharded page files inside pages/");

    // Now re-open connection to verify we can query the data
    let conn2 = Connection::open(&db_path)?;
    let count: i64 = conn2.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    assert_eq!(count, 100, "All 100 rows should be queryable from the sharded database");

    Ok(())
}
