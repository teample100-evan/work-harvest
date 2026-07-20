use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn run(mut command: Command, args: &[&str], input: Option<&[u8]>) -> Output {
    let mut child = command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    child.wait_with_output().unwrap()
}

fn rust_cli(args: &[&str], input: Option<&[u8]>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wh"));
    command.current_dir(repository_root());
    run(command, args, input)
}

fn node_cli(args: &[&str]) -> Output {
    let repository = repository_root();
    let mut command = Command::new("node");
    command
        .arg(repository.join("bin/wh.js"))
        .current_dir(repository);
    run(command, args, None)
}

fn work_item_input() -> Value {
    json!({
        "id": "AUTH-142",
        "project_id": "jajak-front",
        "title": "인증 시스템 개선",
        "objective": "토큰 만료 시 요청을 안전하게 재시도한다.",
        "desired_outcomes": ["인증 갱신 동작을 테스트로 검증한다."],
        "classification": {
            "initiative_id": "authentication",
            "work_types": ["testing"],
            "tags": ["auth"]
        },
        "created_at": "2026-07-13T09:00:00.000Z",
        "updated_at": "2026-07-13T09:00:00.000Z",
        "context": {
            "current_state": "인증 테스트 작업을 시작하기 전이다.",
            "next_steps": ["기본 성공 경로 테스트 작성"]
        }
    })
}

fn checkpoint_input() -> Value {
    json!({
        "id": "CP-20260713-001",
        "work_item_id": "AUTH-142",
        "kind": "progress",
        "captured_at": "2026-07-13T18:10:00+09:00",
        "source": {
            "agent": "codex",
            "surface": "desktop",
            "session_ref": "session-test",
            "task_title": "인증 테스트 코드 작성"
        },
        "title": "인증 테스트 진행",
        "summary": "인증 갱신 성공 경로를 검증했다.",
        "activities": ["refresh token 갱신 테스트를 추가했다."],
        "verifications": [{
            "type": "test",
            "description": "인증 테스트",
            "status": "passed",
            "command": "pnpm test auth",
            "evidence_refs": ["tests/auth.test.ts"]
        }],
        "evidence": {
            "files": ["tests/auth.test.ts"],
            "commands": ["pnpm test auth"]
        },
        "next_steps": ["동시 요청 테스트 작성"],
        "context_update": {
            "current_state": "기본 성공 경로를 검증했고 동시 요청 테스트가 남아 있다.",
            "next_steps": ["동시 요청 테스트 작성"]
        }
    })
}

#[test]
fn read_commands_match_node_cli_byte_for_byte_on_examples() {
    let commands = [
        vec!["work-item", "list", "--root", "examples", "--json"],
        vec![
            "work-item",
            "list",
            "--compact",
            "--root",
            "examples",
            "--json",
        ],
        vec![
            "work-item",
            "show",
            "AUTH-142",
            "--root",
            "examples",
            "--json",
        ],
        vec![
            "work-item",
            "show",
            "AUTH-142",
            "--compact",
            "--root",
            "examples",
            "--json",
        ],
        vec![
            "checkpoint",
            "last",
            "--work-item",
            "AUTH-142",
            "--root",
            "examples",
            "--json",
        ],
        vec![
            "checkpoint",
            "boundary",
            "--work-item",
            "AUTH-142",
            "--root",
            "examples",
            "--json",
        ],
        vec!["validate", "--root", "examples", "--json"],
    ];
    for args in commands {
        let node = node_cli(&args);
        let rust = rust_cli(&args, None);
        assert!(
            node.status.success(),
            "{}",
            String::from_utf8_lossy(&node.stderr)
        );
        assert!(
            rust.status.success(),
            "{}",
            String::from_utf8_lossy(&rust.stderr)
        );
        assert_eq!(rust.stdout, node.stdout, "command: {}", args.join(" "));
        assert_eq!(rust.stderr, node.stderr, "command: {}", args.join(" "));
    }
}

#[test]
fn rust_cli_runs_the_complete_write_and_report_flow() {
    let directory = tempdir().unwrap();
    let root = directory.path().to_str().unwrap();
    let work_item = serde_json::to_vec(&work_item_input()).unwrap();
    let created = rust_cli(
        &[
            "work-item",
            "create",
            "--input",
            "-",
            "--root",
            root,
            "--json",
        ],
        Some(&work_item),
    );
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let created: Value = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(created["work_item"]["id"], "AUTH-142");
    assert_eq!(created.as_object().unwrap().len(), 3);

    let checkpoint = serde_json::to_vec(&checkpoint_input()).unwrap();
    let captured = rust_cli(
        &[
            "checkpoint",
            "capture",
            "--input",
            "-",
            "--root",
            root,
            "--json",
        ],
        Some(&checkpoint),
    );
    assert!(
        captured.status.success(),
        "{}",
        String::from_utf8_lossy(&captured.stderr)
    );
    let captured: Value = serde_json::from_slice(&captured.stdout).unwrap();
    assert_eq!(captured["checkpoint"]["id"], "CP-20260713-001");
    assert_eq!(captured.as_object().unwrap().len(), 4);

    let compact_checkpoint = serde_json::to_vec(&checkpoint_input()).unwrap();
    let compact_root = tempdir().unwrap();
    let compact_root_path = compact_root.path().to_str().unwrap();
    let compact_work_item = serde_json::to_vec(&work_item_input()).unwrap();
    let compact_created = rust_cli(
        &[
            "work-item",
            "create",
            "--input",
            "-",
            "--root",
            compact_root_path,
            "--json",
        ],
        Some(&compact_work_item),
    );
    assert!(compact_created.status.success());
    let compact_captured = rust_cli(
        &[
            "checkpoint",
            "capture",
            "--input",
            "-",
            "--compact",
            "--root",
            compact_root_path,
            "--json",
        ],
        Some(&compact_checkpoint),
    );
    assert!(
        compact_captured.status.success(),
        "{}",
        String::from_utf8_lossy(&compact_captured.stderr)
    );
    let compact_captured: Value = serde_json::from_slice(&compact_captured.stdout).unwrap();
    assert_eq!(compact_captured["checkpoint"]["id"], "CP-20260713-001");
    assert!(compact_captured["checkpoint"].get("activities").is_none());
    assert!(compact_captured["context"].get("current_state").is_none());

    let shown = rust_cli(
        &["work-item", "show", "AUTH-142", "--root", root, "--json"],
        None,
    );
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&shown.stdout).unwrap()["last_checkpoint"]["checkpoint"]["id"],
        "CP-20260713-001"
    );

    let report = rust_cli(
        &[
            "report",
            "performance-note",
            "--work-item",
            "AUTH-142",
            "--root",
            root,
            "--json",
        ],
        None,
    );
    assert!(
        report.status.success(),
        "{}",
        String::from_utf8_lossy(&report.stderr)
    );
    let report: Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(report.as_object().unwrap().len(), 5);
    assert_eq!(report["redacted_checkpoint_count"], 0);
    assert_eq!(report["excluded_checkpoint_count"], 0);
    let report_path = report["paths"]["report"].as_str().unwrap();
    let markdown = fs::read_to_string(directory.path().join(report_path)).unwrap();
    assert!(markdown.contains("# 1. 작업 개요"));
    assert!(markdown.contains("# 13. 업무 요약"));

    let validation = rust_cli(&["validate", "--root", root, "--json"], None);
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );
    let validation: Value = serde_json::from_slice(&validation.stdout).unwrap();
    assert_eq!(validation["datasets"][0]["counts"]["checkpoints"], 1);
}

#[test]
fn rust_cli_accepts_nested_yaml_and_preserves_usage_errors() {
    let directory = tempdir().unwrap();
    let root = directory.path().to_str().unwrap();
    let yaml = r#"
id: DOC-7
project_id: docs
title: 운영 배포 가이드
objective: 배포와 복구 절차를 문서화한다.
created_at: 2026-07-13T09:00:00.000Z
updated_at: 2026-07-13T09:00:00.000Z
classification:
  work_types:
    - documentation
    - operation
context:
  current_state: 배포 절차 초안을 작성하기 전이다.
  next_steps:
    - 배포 순서 작성
"#;
    let created = rust_cli(
        &[
            "work-item",
            "create",
            "--input",
            "-",
            "--root",
            root,
            "--json",
        ],
        Some(yaml.as_bytes()),
    );
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let created: Value = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(
        created["work_item"]["classification"]["work_types"],
        json!(["documentation", "operation"])
    );

    let invalid = rust_cli(
        &[
            "report",
            "performance-note",
            "--work-item",
            "DOC-7",
            "--output",
            "reports/DOC-7.txt",
            "--root",
            root,
        ],
        None,
    );
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(stderr.contains("Report output must be a .md file"));
    assert!(stderr.contains("Work Harvest CLI"));
}
