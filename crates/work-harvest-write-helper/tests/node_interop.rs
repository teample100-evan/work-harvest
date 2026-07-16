use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;
use work_harvest_core::DataRootWriter;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn helper_path() -> &'static str {
    env!("CARGO_BIN_EXE_work-harvest-write-helper")
}

fn run_cli(root: &Path, helper: &Path, args: &[&str], input: Option<&Value>) -> Output {
    let repository = repository_root();
    let mut child = Command::new("node")
        .arg(repository.join("bin/wh.js"))
        .args(args)
        .args(["--root", root.to_str().unwrap()])
        .current_dir(&repository)
        .env("WORK_HARVEST_WRITE_HELPER", helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(serde_json::to_string(input).unwrap().as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

fn work_item_input() -> Value {
    json!({
        "id": "AUTH-142",
        "project_id": "jajak-front",
        "title": "인증 시스템 개선",
        "objective": "토큰 만료 시 요청을 안전하게 재시도한다.",
        "classification": { "work_types": ["testing"] },
        "context": { "current_state": "인증 테스트 작업을 시작하기 전이다." }
    })
}

fn checkpoint_input() -> Value {
    json!({
        "id": "CP-20260714-001",
        "work_item_id": "AUTH-142",
        "captured_at": "2026-07-14T12:00:00+09:00",
        "title": "인증 테스트 진행",
        "summary": "기본 성공 경로를 검증했다.",
        "activities": ["인증 테스트를 추가했다."],
        "context_update": {
            "current_state": "동시 요청 테스트가 남아 있다."
        }
    })
}

#[test]
fn node_writer_respects_the_rust_advisory_lock() {
    let directory = tempdir().unwrap();
    let _writer = DataRootWriter::acquire(directory.path()).unwrap();

    let output = run_cli(
        directory.path(),
        Path::new(helper_path()),
        &["work-item", "create", "--input", "-"],
        Some(&work_item_input()),
    );

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Another Work Harvest writer is using data root")
    );
    assert!(!directory.path().join("work-items/AUTH-142").exists());
}

#[test]
fn node_performance_note_uses_the_same_advisory_lock() {
    let directory = tempdir().unwrap();
    let helper = Path::new(helper_path());
    let created = run_cli(
        directory.path(),
        helper,
        &["work-item", "create", "--input", "-"],
        Some(&work_item_input()),
    );
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let _writer = DataRootWriter::acquire(directory.path()).unwrap();

    let output = run_cli(
        directory.path(),
        helper,
        &["report", "performance-note", "--work-item", "AUTH-142"],
        None,
    );

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Another Work Harvest writer is using data root")
    );
    assert!(!directory.path().join("reports").exists());
}

#[cfg(unix)]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\"'\"'"))
}

#[cfg(unix)]
#[test]
fn stale_node_checkpoint_revision_writes_none_of_its_five_files() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let helper = Path::new(helper_path());
    let created = run_cli(
        directory.path(),
        helper,
        &["work-item", "create", "--input", "-"],
        Some(&work_item_input()),
    );
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );

    let work_item_path = directory.path().join("work-items/AUTH-142/work-item.json");
    let context_data_path = directory.path().join("work-items/AUTH-142/context.json");
    let context_markdown_path = directory.path().join("work-items/AUTH-142/context.md");
    let work_item_before = fs::read(&work_item_path).unwrap();
    let context_data_before = fs::read(&context_data_path).unwrap();

    let wrapper = directory.path().join("change-context-before-helper.sh");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nprintf '%s\\n' 'external editor change' > {}\nexec {}\n",
            shell_quote(&context_markdown_path),
            shell_quote(helper),
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();

    let captured = run_cli(
        directory.path(),
        &wrapper,
        &["checkpoint", "capture", "--input", "-"],
        Some(&checkpoint_input()),
    );

    assert!(!captured.status.success());
    let stderr = String::from_utf8_lossy(&captured.stderr);
    assert!(
        stderr.contains("File changed since it was read"),
        "{stderr}"
    );
    assert!(stderr.contains("context.md"), "{stderr}");
    assert_eq!(fs::read(&work_item_path).unwrap(), work_item_before);
    assert_eq!(fs::read(&context_data_path).unwrap(), context_data_before);
    assert_eq!(
        fs::read_to_string(&context_markdown_path).unwrap(),
        "external editor change\n"
    );
    assert!(!directory.path().join("records/2026/07/14").exists());
}
