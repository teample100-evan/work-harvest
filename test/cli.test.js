import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const cliPath = path.join(repositoryRoot, "bin", "wh.js");

function run(args, input) {
  return spawnSync(process.execPath, [cliPath, ...args], {
    cwd: repositoryRoot,
    input: input ? JSON.stringify(input) : undefined,
    encoding: "utf8",
  });
}

function createPayload(id = "AUTH-142") {
  return {
    id,
    project_id: "jajak-front",
    title: "인증 시스템 개선",
    objective: "토큰 만료 시 요청을 안전하게 재시도한다.",
    desired_outcomes: ["인증 갱신 동작을 테스트로 검증한다."],
    classification: {
      initiative_id: "authentication",
      work_types: ["testing"],
      tags: ["auth"],
    },
    repositories: [],
    links: [],
    context: {
      current_state: "인증 테스트 작업을 시작하기 전이다.",
      next_steps: ["기본 성공 경로 테스트 작성"],
    },
  };
}

function checkpointPayload(kind = "progress") {
  return {
    id: kind === "final" ? "CP-20260713-999" : "CP-20260713-001",
    work_item_id: "AUTH-142",
    kind,
    captured_at: "2026-07-13T18:10:00+09:00",
    source: {
      agent: "codex",
      surface: "desktop",
      session_ref: "session-test",
      task_title: "인증 테스트 코드 작성",
    },
    title: kind === "final" ? "인증 테스트 완료" : "인증 테스트 진행",
    summary: "인증 갱신 성공 경로를 검증했다.",
    activities: ["refresh token 갱신 테스트를 추가했다."],
    verifications: [
      {
        type: "test",
        description: "인증 테스트",
        status: "passed",
        command: "pnpm test auth",
        evidence_refs: ["tests/auth.test.ts"],
      },
    ],
    evidence: {
      files: ["tests/auth.test.ts"],
      commands: ["pnpm test auth"],
    },
    outcomes:
      kind === "final"
        ? [
            {
              description: "인증 갱신 테스트 스위트를 완료했다.",
              impact: null,
              evidence_refs: ["tests/auth.test.ts"],
            },
          ]
        : [],
    next_steps: kind === "final" ? [] : ["동시 요청 테스트 작성"],
    context_update: {
      current_state:
        kind === "final"
          ? "인증 갱신 테스트 스위트를 완료했다."
          : "기본 성공 경로를 검증했고 동시 요청 테스트가 남아 있다.",
      verification: {
        completed: ["인증 갱신 기본 성공 경로 테스트 통과"],
        pending: kind === "final" ? [] : ["동시 요청 테스트"],
      },
      next_steps: kind === "final" ? [] : ["동시 요청 테스트 작성"],
      files: ["tests/auth.test.ts"],
    },
  };
}

test("work item creation, checkpoint capture, and validation form one flow", () => {
  const root = mkdtempSync(path.join(os.tmpdir(), "work-harvest-test-"));
  try {
    const created = run(
      ["work-item", "create", "--input", "-", "--root", root, "--json"],
      createPayload(),
    );
    assert.equal(created.status, 0, created.stderr);
    const createdResult = JSON.parse(created.stdout);
    assert.equal(createdResult.work_item.id, "AUTH-142");

    const duplicate = run(
      ["work-item", "create", "--input", "-", "--root", root],
      createPayload(),
    );
    assert.notEqual(duplicate.status, 0);
    assert.match(duplicate.stderr, /already exists/);

    const captured = run(
      ["checkpoint", "capture", "--input", "-", "--root", root, "--json"],
      checkpointPayload(),
    );
    assert.equal(captured.status, 0, captured.stderr);
    const capturedResult = JSON.parse(captured.stdout);
    assert.equal(capturedResult.checkpoint.id, "CP-20260713-001");
    assert.equal(
      capturedResult.paths.checkpoint,
      path.join("records", "2026", "07", "13", "CP-20260713-001.json"),
    );
    const checkpointMarkdown = readFileSync(
      path.join(root, capturedResult.paths.checkpoint_markdown),
      "utf8",
    );
    assert.match(checkpointMarkdown, /# 인증 테스트 진행/);
    assert.match(checkpointMarkdown, /pnpm test auth/);

    const context = readFileSync(
      path.join(root, "work-items", "AUTH-142", "context.md"),
      "utf8",
    );
    assert.match(context, /last_checkpoint_id: "CP-20260713-001"/);
    assert.match(context, /기본 성공 경로를 검증했고 동시 요청 테스트가 남아 있다/);

    const listed = run(["work-item", "list", "--root", root, "--json"]);
    assert.equal(listed.status, 0, listed.stderr);
    assert.equal(JSON.parse(listed.stdout).work_items[0].id, "AUTH-142");

    const shown = run([
      "work-item",
      "show",
      "AUTH-142",
      "--root",
      root,
      "--json",
    ]);
    assert.equal(shown.status, 0, shown.stderr);
    assert.equal(
      JSON.parse(shown.stdout).last_checkpoint.checkpoint.id,
      "CP-20260713-001",
    );

    const last = run([
      "checkpoint",
      "last",
      "--work-item",
      "AUTH-142",
      "--root",
      root,
      "--json",
    ]);
    assert.equal(last.status, 0, last.stderr);
    assert.equal(
      JSON.parse(last.stdout).checkpoint.id,
      "CP-20260713-001",
    );

    const report = run([
      "report",
      "performance-note",
      "--work-item",
      "AUTH-142",
      "--root",
      root,
      "--json",
    ]);
    assert.equal(report.status, 0, report.stderr);
    const reportResult = JSON.parse(report.stdout);
    assert.equal(reportResult.checkpoint_count, 1);
    assert.equal(
      reportResult.paths.report,
      path.join("reports", "performance-notes", "AUTH-142-20260713.md"),
    );
    const reportMarkdown = readFileSync(
      path.join(root, reportResult.paths.report),
      "utf8",
    );
    assert.match(reportMarkdown, /# 1\. 작업 개요/);
    assert.match(reportMarkdown, /# 13\. 업무 요약/);
    assert.match(reportMarkdown, /인증 테스트 \(passed\)/);

    const duplicateReport = run([
      "report",
      "performance-note",
      "--work-item",
      "AUTH-142",
      "--root",
      root,
    ]);
    assert.notEqual(duplicateReport.status, 0);
    assert.match(duplicateReport.stderr, /already exists/);

    const validation = run(["validate", "--root", root, "--json"]);
    assert.equal(validation.status, 0, validation.stderr);
    const validationResult = JSON.parse(validation.stdout);
    assert.equal(validationResult.valid, true);
    assert.deepEqual(validationResult.datasets[0].counts, {
      work_items: 1,
      contexts: 1,
      checkpoints: 1,
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("invalid empty checkpoint is rejected without writing a record", () => {
  const root = mkdtempSync(path.join(os.tmpdir(), "work-harvest-test-"));
  try {
    const created = run(
      ["work-item", "create", "--input", "-", "--root", root],
      createPayload(),
    );
    assert.equal(created.status, 0, created.stderr);

    const invalid = run(
      ["checkpoint", "capture", "--input", "-", "--root", root],
      {
        work_item_id: "AUTH-142",
        title: "빈 체크포인트",
        summary: "구체적인 기록 내용이 없다.",
      },
    );
    assert.notEqual(invalid.status, 0);
    assert.match(invalid.stderr, /Checkpoint validation failed/);

    const validation = run(["validate", "--root", root, "--json"]);
    assert.equal(validation.status, 0, validation.stderr);
    assert.equal(JSON.parse(validation.stdout).datasets[0].counts.checkpoints, 0);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("final checkpoint completes the work item and updates context metadata", () => {
  const root = mkdtempSync(path.join(os.tmpdir(), "work-harvest-test-"));
  try {
    const created = run(
      ["work-item", "create", "--input", "-", "--root", root],
      createPayload(),
    );
    assert.equal(created.status, 0, created.stderr);

    const finalWithoutOutcome = checkpointPayload("final");
    finalWithoutOutcome.outcomes = [];
    const rejected = run(
      ["checkpoint", "capture", "--input", "-", "--root", root],
      finalWithoutOutcome,
    );
    assert.notEqual(rejected.status, 0);
    assert.match(rejected.stderr, /Checkpoint validation failed/);

    const captured = run(
      ["checkpoint", "capture", "--input", "-", "--root", root],
      checkpointPayload("final"),
    );
    assert.equal(captured.status, 0, captured.stderr);

    const workItem = JSON.parse(
      readFileSync(
        path.join(root, "work-items", "AUTH-142", "work-item.json"),
        "utf8",
      ),
    );
    assert.equal(workItem.status, "completed");
    assert.equal(workItem.completed_at, "2026-07-13T18:10:00+09:00");

    const context = readFileSync(
      path.join(root, "work-items", "AUTH-142", "context.md"),
      "utf8",
    );
    assert.match(context, /status: "completed"/);
    assert.match(context, /last_checkpoint_id: "CP-20260713-999"/);
    assert.match(context, /인증 갱신 테스트 스위트를 완료했다/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("retrospective non-code work preserves capture time and user-supplied work range", () => {
  const root = mkdtempSync(path.join(os.tmpdir(), "work-harvest-test-"));
  try {
    const workItem = createPayload("DOC-7");
    workItem.title = "운영 배포 가이드 작성";
    workItem.objective = "배포 절차와 장애 복구 순서를 문서화한다.";
    workItem.classification.work_types = ["documentation", "operation"];
    workItem.context.current_state = "사후 기록을 시작하기 전이다.";

    const created = run(
      ["work-item", "create", "--input", "-", "--root", root],
      workItem,
    );
    assert.equal(created.status, 0, created.stderr);

    const captured = run(
      ["checkpoint", "capture", "--input", "-", "--root", root, "--json"],
      {
        id: "CP-20260713-DOC7",
        work_item_id: "DOC-7",
        kind: "backfill",
        captured_at: "2026-07-13T12:00:00+09:00",
        source: {
          agent: "manual",
          surface: "desktop",
          session_ref: null,
          task_title: "지난주 문서 작업 기록",
        },
        work_period: {
          start: "2026-07-06",
          end: "2026-07-08",
          precision: "range",
          basis: ["user"],
          timezone: "Asia/Seoul",
        },
        title: "배포 가이드 초안 작성",
        summary: "사용자 설명에 따르면 배포 가이드 초안을 작성했다.",
        activities: ["배포 절차와 장애 복구 순서를 문서화했다."],
        verifications: [
          {
            type: "review",
            description: "배포 가이드 내용 검토",
            status: "not_run",
            command: null,
            evidence_refs: [],
          },
        ],
        outcomes: [
          {
            description: "배포 가이드 초안이 작성됐다고 사용자가 보고했다.",
            impact: "사용자 제공 설명 기준이며 독립적으로 확인하지 않았다.",
            evidence_refs: [],
          },
        ],
        next_steps: ["문서 링크를 연결하고 내용을 검토한다."],
        context_update: {
          current_state: "배포 가이드 초안을 사후 기록했으며 문서 확인이 남아 있다.",
          verification: {
            completed: [],
            pending: ["배포 가이드 문서 확인"],
          },
          next_steps: ["문서 링크 연결 및 내용 검토"],
          risks: ["현재 기록은 사용자 설명에 기반한다."],
        },
      },
    );
    assert.equal(captured.status, 0, captured.stderr);
    const result = JSON.parse(captured.stdout);
    assert.equal(result.checkpoint.kind, "backfill");
    assert.equal(result.checkpoint.source.agent, "manual");
    assert.deepEqual(result.checkpoint.work_period, {
      start: "2026-07-06",
      end: "2026-07-08",
      precision: "range",
      basis: ["user"],
      timezone: "Asia/Seoul",
    });
    assert.equal(
      result.paths.checkpoint,
      path.join("records", "2026", "07", "13", "CP-20260713-DOC7.json"),
    );

    const validation = run(["validate", "--root", root, "--json"]);
    assert.equal(validation.status, 0, validation.stderr);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
