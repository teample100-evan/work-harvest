---
schema_version: "1.0"
id: "<CHECKPOINT_ID>"
work_item_id: "<WORK_ITEM_ID>"
project_id: "<PROJECT_ID>"
kind: "progress"
source:
  agent: "codex"
  surface: "desktop"
  session_ref: null
  task_title: null
captured_at: "<ISO-8601 timestamp>"
work_period:
  start: "<ISO-8601 date or timestamp>"
  end: "<ISO-8601 date or timestamp>"
  precision: "day"
  basis:
    - "checkpoint"
  timezone: "Asia/Seoul"
title: "<체크포인트 제목>"
status_after: "in_progress"
related_checkpoint_ids: []
correction_of: null
confidentiality: "normal"
---

# <체크포인트 제목>

## 요약

<마지막 체크포인트 이후 진행한 작업을 짧게 요약한다.>

## 진행한 작업

- <구체적인 변경 또는 수행 내용>

## 결정 및 이유

- 결정: <결정 내용>
  - 이유: <선택 근거>
  - 상태: <proposed | accepted | superseded>

## 검증

- 유형: <test | build | lint | manual | measurement | review | other>
  - 설명: <무엇을 확인했는지>
  - 상태: <passed | failed | partial | not_run>
  - 명령: `<command 또는 null>`
  - 근거: <로그, 파일, 커밋 등의 참조>

## 결과와 영향

- 결과: <확인된 결과>
  - 영향: <확인된 영향 또는 확인되지 않았으면 null>
  - 근거: <결과를 뒷받침하는 참조>

## 차단 요소

- <없으면 비워 둔다.>

## 다음 작업

- <다음 체크포인트까지 수행할 작업>

## 근거

- 커밋: <sha>
- PR: <PR 참조>
- 이슈: <이슈 참조>
- 파일: `<path>`
- 명령: `<command>`
- URL: <관련 링크>

## Git 상태

- 저장소: `<repository>`
- 브랜치: `<branch 또는 null>`
- 이전 HEAD: `<sha 또는 null>`
- 현재 HEAD: `<sha 또는 null>`
- 미커밋 변경: `<true | false | null>`
