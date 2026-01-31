//! Integration tests for batch rename functionality
//!
//! Verifies that the rename_all tool supports the "targets" parameter for batch operations.

use crate::test_helpers::*;
use serde_json::json;

#[tokio::test]
async fn test_batch_rename_files() {
    run_tool_test(
        &[
            ("file1.txt", "content1"),
            ("file2.txt", "content2"),
            ("subdir/file3.txt", "content3"),
        ],
        "rename_all",
        |ws| {
            json!({
                "targets": [
                    {
                        "kind": "file",
                        "filePath": ws.absolute_path("file1.txt").to_str().unwrap(),
                        "newName": ws.absolute_path("file1_renamed.txt").to_str().unwrap()
                    },
                    {
                        "kind": "file",
                        "filePath": ws.absolute_path("file2.txt").to_str().unwrap(),
                        "newName": ws.absolute_path("file2_renamed.txt").to_str().unwrap()
                    },
                    {
                        "kind": "file",
                        "filePath": ws.absolute_path("subdir/file3.txt").to_str().unwrap(),
                        "newName": ws.absolute_path("subdir/file3_renamed.txt").to_str().unwrap()
                    }
                ]
            })
        },
        |ws| {
            assert!(!ws.file_exists("file1.txt"), "file1.txt should be gone");
            assert!(
                ws.file_exists("file1_renamed.txt"),
                "file1_renamed.txt should exist"
            );
            assert_eq!(ws.read_file("file1_renamed.txt"), "content1");

            assert!(!ws.file_exists("file2.txt"), "file2.txt should be gone");
            assert!(
                ws.file_exists("file2_renamed.txt"),
                "file2_renamed.txt should exist"
            );
            assert_eq!(ws.read_file("file2_renamed.txt"), "content2");

            assert!(
                !ws.file_exists("subdir/file3.txt"),
                "subdir/file3.txt should be gone"
            );
            assert!(
                ws.file_exists("subdir/file3_renamed.txt"),
                "subdir/file3_renamed.txt should exist"
            );
            assert_eq!(ws.read_file("subdir/file3_renamed.txt"), "content3");
            Ok(())
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_batch_rename_directories() {
    run_tool_test(
        &[
            ("dir1/file1.txt", "content1"),
            ("dir2/file2.txt", "content2"),
        ],
        "rename_all",
        |ws| {
            json!({
                "targets": [
                    {
                        "kind": "directory",
                        "filePath": ws.absolute_path("dir1").to_str().unwrap(),
                        "newName": ws.absolute_path("dir1_new").to_str().unwrap()
                    },
                    {
                        "kind": "directory",
                        "filePath": ws.absolute_path("dir2").to_str().unwrap(),
                        "newName": ws.absolute_path("dir2_new").to_str().unwrap()
                    }
                ]
            })
        },
        |ws| {
            assert!(!ws.file_exists("dir1/file1.txt"));
            assert!(ws.file_exists("dir1_new/file1.txt"));
            assert_eq!(ws.read_file("dir1_new/file1.txt"), "content1");

            assert!(!ws.file_exists("dir2/file2.txt"));
            assert!(ws.file_exists("dir2_new/file2.txt"));
            assert_eq!(ws.read_file("dir2_new/file2.txt"), "content2");
            Ok(())
        },
    )
    .await
    .unwrap();
}
