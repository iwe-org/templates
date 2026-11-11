use std::fs;

use iwe_template::{
    pack, read_dir, unpack, validate_dir, validate_entries, Origin, Violation, MAX_FILE_BYTES,
};

use super::fixture::{valid_template, write_tree};

#[test]
fn a_checkout_on_disk_passes_and_its_git_folder_is_skipped() {
    let root = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    fs::create_dir_all(root.path().join(".git/objects")).unwrap();
    fs::write(root.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    assert_eq!(validate_dir(root.path(), None).unwrap(), vec![]);
    let read = read_dir(root.path()).unwrap();
    assert!(read.iter().all(|entry| !entry.path.starts_with(".git/")));
    assert_eq!(read.len(), valid_template().len() - 1);
}

#[test]
fn a_nested_git_folder_fails() {
    let root = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    fs::create_dir_all(root.path().join("data/.git")).unwrap();
    fs::write(root.path().join("data/.git/HEAD"), "x\n").unwrap();
    assert_eq!(
        validate_dir(root.path(), None).unwrap(),
        vec![Violation::DotPathNotAllowed {
            path: "data/.git/HEAD".to_string()
        }]
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_on_disk_fails() {
    let root = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    std::os::unix::fs::symlink("/etc/hosts", root.path().join("data/hosts.md")).unwrap();
    std::os::unix::fs::symlink("notes", root.path().join("data/alias")).unwrap();
    assert_eq!(
        validate_dir(root.path(), None).unwrap(),
        vec![
            Violation::Symlink {
                path: "data/alias".to_string()
            },
            Violation::Symlink {
                path: "data/hosts.md".to_string()
            },
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_hard_link_on_disk_fails() {
    let root = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    fs::hard_link(
        root.path().join("data/index.md"),
        root.path().join("data/twin.md"),
    )
    .unwrap();
    assert_eq!(
        validate_dir(root.path(), None).unwrap(),
        vec![
            Violation::HardLink {
                path: "data/index.md".to_string()
            },
            Violation::HardLink {
                path: "data/twin.md".to_string()
            },
        ]
    );
}

#[test]
fn an_oversized_file_on_disk_fails_with_its_real_size() {
    let root = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    fs::write(
        root.path().join("data/big.md"),
        vec![b'a'; MAX_FILE_BYTES as usize + 7],
    )
    .unwrap();
    assert_eq!(
        validate_dir(root.path(), None).unwrap(),
        vec![Violation::FileTooLarge {
            path: "data/big.md".to_string(),
            size: MAX_FILE_BYTES + 7,
        }]
    );
}

#[test]
fn a_folder_packs_unpacks_and_validates() {
    let root = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    let origin = Origin::default();
    let packed = pack(root.path(), &origin).unwrap();
    assert_eq!(packed, pack(root.path(), &origin).unwrap());
    assert_eq!(
        validate_entries(&unpack(&packed.archive).unwrap(), None),
        vec![]
    );
}

#[test]
fn a_missing_folder_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    assert!(validate_dir(&root.path().join("absent"), None).is_err());
}
