import path from "node:path";
import { CliError } from "./errors.js";
import { pathExists, writeTextExclusive } from "./io.js";
import { resolveWithinRoot } from "./paths.js";
import { listCheckpointsForWorkItem } from "./queries.js";
import { loadWorkItem, readContext } from "./work-items.js";

function bullets(values, fallback = "미확인") {
  return values?.length
    ? values.map((value) => `- ${value}`).join("\n")
    : `- ${fallback}`;
}

function evidence(checkpoint) {
  const entries = Object.entries(checkpoint.evidence ?? {}).flatMap(([kind, values]) =>
    values?.map((value) => `${kind}: ${value}`) ?? [],
  );
  return bullets(entries, "근거 미기록");
}

function verification(checkpoints, type) {
  const matches = checkpoints.flatMap((checkpoint) =>
    checkpoint.verifications
      .filter((item) => item.type === type)
      .map(
        (item) =>
          `${checkpoint.id} — ${item.description} (${item.status})${item.command ? `: \`${item.command}\`` : ""}`,
      ),
  );
  return bullets(matches, "미확인");
}

function renderBatches(checkpoints) {
  if (!checkpoints.length) return "- 체크포인트가 없어 작업 내용을 확인할 수 없습니다.";
  return checkpoints
    .map((entry, index) => {
      const checkpoint = entry.checkpoint;
      return `## 배치 ${index + 1}: ${checkpoint.title}

- 대상: ${checkpoint.work_period.start} ~ ${checkpoint.work_period.end}
- 변경 내용:
${bullets(checkpoint.activities, "미기록")}
- 검증 방법:
${bullets(
  checkpoint.verifications.map(
    (item) => `${item.type}: ${item.description} (${item.status})${item.command ? ` — \`${item.command}\`` : ""}`,
  ),
  "미확인",
)}
- 결과:
${bullets(checkpoint.outcomes.map((item) => item.description), checkpoint.summary)}
- 근거:
${evidence(checkpoint)}`;
    })
    .join("\n\n");
}

export function renderPerformanceNote({ workItem, context, checkpoints }) {
  const repository = context.git.repository ?? workItem.repositories[0]?.url ?? "미확인";
  const branch = context.git.branch ?? "미확인";
  const latest = checkpoints.at(-1)?.checkpoint;
  const checkpointValues = checkpoints.map((entry) => entry.checkpoint);
  const decisions = checkpointValues.flatMap((checkpoint) =>
    checkpoint.decisions.map(
      (item) => `${item.summary} — ${item.rationale} (${item.status})`,
    ),
  );
  const outcomes = checkpointValues.flatMap((checkpoint) =>
    checkpoint.outcomes.map((item) => item.description),
  );
  const risks = [...context.risks, ...checkpointValues.flatMap((checkpoint) => checkpoint.blockers)];

  return `---
work_item_id: ${JSON.stringify(workItem.id)}
project_id: ${JSON.stringify(workItem.project_id)}
generated_from_checkpoints: ${JSON.stringify(checkpoints.map((entry) => entry.checkpoint.id))}
generated_at: ${JSON.stringify(new Date().toISOString())}
---

# ${workItem.title} 성과 노트

> 체크포인트 원본에서 생성한 초안입니다. 작업 규모에 맞지 않는 섹션은 삭제하고, 정량 수치·배포 결과처럼 근거가 없는 내용은 확인 후 보완합니다.

# 1. 작업 개요

- 작업명: ${workItem.title}
- 작업 유형: ${workItem.classification.work_types.join(" / ") || "미확인"}
- 저장소: ${repository}
- 브랜치: ${branch}
- PR: 미확인
- 배포 대상: 미확인
- 작성일: ${latest?.captured_at ?? "미확인"}
- 작성자: ${latest?.source.agent ?? "미확인"}
- 상태: ${workItem.status}
- 관련 문서 또는 체크리스트:
${bullets(workItem.links.map((link) => link.url ?? link), "미확인")}

# 2. 요약

- 한 줄 요약: ${latest?.summary ?? workItem.objective}
- 핵심 결과:
${bullets(outcomes, "미확인")}
- 영향 범위: ${workItem.desired_outcomes.join(" / ") || "미확인"}
- 최종 상태: ${context.current_state}

# 3. 작업 배경

- 기존 문제: ${workItem.objective}
- 사용자 경험에서의 문제: 미확인
- 개발 경험에서의 문제: 미확인
- 지금 정리해야 하는 이유: ${workItem.objective}

# 4. 작업 목표

${bullets(workItem.desired_outcomes, workItem.objective)}
- 제외 범위: 미확인

# 5. 작업 기준

- 전환 또는 수정 기준: 미확인
- 유지 기준: 미확인
- 제외 기준: 미확인
- 의사결정 기준:
${bullets(decisions, "미확인")}

# 6. 진행한 작업

${renderBatches(checkpoints)}

# 7. 작업 후 확인된 내용

## 정량 성과

- 대상 수: 미확인 → 미확인
- 코드 라인 수: 미확인 → 미확인
- 파일 수: 미확인 → 미확인
- 중복 또는 분기 감소: 미확인
- 오류 또는 경고 변화: 미확인

## 정성 성과

- DX 개선: 미확인
- UI/UX 개선: 미확인
- 유지보수성 개선: 미확인
- 정책 또는 문서화 개선:
${bullets(decisions, "미확인")}

## 예외 처리

- 예외 대상: 미확인
- 유지 이유: 미확인
- 후속 판단 조건: 미확인

# 8. 기대되는 변화

- 사용자 경험: 미확인
- 개발 경험: 미확인
- 운영 또는 배포 관점: 미확인
- 장기 유지보수 관점: 미확인

# 9. 확인한 검증

- 자동 검증:
${verification(checkpointValues, "test")}
- 수동 검증:
${verification(checkpointValues, "manual")}
- 브라우저 확인:
${verification(checkpointValues, "review")}
- 배포 확인: 미확인
- 미실행 검증과 사유:
${bullets(
  checkpointValues.flatMap((checkpoint) =>
    checkpoint.verifications
      .filter((item) => item.status === "not_run")
      .map((item) => `${item.description} (${item.type})`),
  ),
  "미확인",
)}

# 10. 남은 주의사항

- 회귀 가능성:
${bullets(risks, "미확인")}
- 예외 케이스: 미확인
- 모니터링 포인트: 미확인
- 롤백 또는 복구 시 고려사항: 미확인

# 11. 후속 확인 사항

${(context.next_steps.length ? context.next_steps : ["후속 확인 필요"]).map((item) => `- [ ] ${item}`).join("\n")}
- [ ] 문서 또는 정책 반영
- [ ] 운영 환경 확인

# 12. 정리

- 이번 작업에서 확정된 기준:
${bullets(decisions, "미확인")}
- 다음 작업자가 따라야 할 방식: ${context.current_state}
- 반복해서 재사용할 수 있는 패턴: 미확인

# 13. 업무 요약

- 공유용 한 문장: ${latest?.summary ?? workItem.objective}
- PR 또는 슬랙용 요약: ${outcomes[0] ?? "미확인"}
- 릴리즈 노트 후보: ${outcomes[0] ?? "미확인"}

# 참고 링크

- PR: ${latest?.evidence.pull_requests.join(", ") || "미확인"}
- 체크리스트: 미확인
- 관련 노트: ${checkpoints.map((entry) => entry.paths.markdown).join(", ") || "미확인"}
- 배포 또는 CI 링크: ${latest?.evidence.urls.join(", ") || "미확인"}
`;
}

export async function createPerformanceNote({ root, validators, workItemId, output }) {
  const { workItem } = await loadWorkItem(root, workItemId);
  const { context } = await readContext(root, workItem, validators);
  const checkpoints = await listCheckpointsForWorkItem({ root, validators, workItemId });
  const date = (checkpoints.at(-1)?.checkpoint.work_period.end ?? new Date().toISOString().slice(0, 10)).replaceAll("-", "");
  const relativePath = output ?? `reports/performance-notes/${workItem.id}-${date}.md`;
  const reportPath = resolveWithinRoot(root, relativePath);
  if (path.extname(reportPath) !== ".md") {
    throw new CliError("Report output must be a .md file", { exitCode: 2 });
  }
  if (await pathExists(reportPath)) {
    throw new CliError(`Performance note already exists: ${path.relative(root, reportPath)}`);
  }
  await writeTextExclusive(reportPath, renderPerformanceNote({ workItem, context, checkpoints }));
  return {
    work_item: workItem,
    checkpoint_count: checkpoints.length,
    paths: { report: path.relative(root, reportPath) },
  };
}
