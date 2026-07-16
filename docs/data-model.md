# Work Harvest 데이터 모델

이 문서는 [작업 방법론](./work-method.md)을 구현하기 위한 버전 1.0 데이터 계약을 설명한다.

## 원본과 파생 데이터

| 데이터 | 변경 정책 | 역할 |
| --- | --- | --- |
| 업무 항목 메타데이터 | 상태 변화에 따라 갱신 | 세션을 넘어 유지되는 업무 식별과 분류 |
| `work-items/<id>/context.json` | 현재 상태로 갱신 | 인수인계 상태의 구조화된 원본 |
| `work-items/<id>/context.md` | 재생성 가능 | 사람이 읽는 인수인계 문서 |
| 체크포인트 | append-only | 특정 구간의 작업 이력과 근거 |
| 자동 활동 이벤트 | append-only | 날짜와 실행 근거 보조 |
| 성과 노트 | 생성 후 수동 편집 가능 | 하나의 업무 항목과 체크포인트를 성과 템플릿으로 정리 |
| 기간 보고서 | 재생성 가능 | 기간별 업무 항목 집계 |

## 스키마

- [`schemas/work-item.schema.json`](../schemas/work-item.schema.json): 업무 항목
- [`schemas/work-context.schema.json`](../schemas/work-context.schema.json): 현재 업무 context
- [`schemas/checkpoint.schema.json`](../schemas/checkpoint.schema.json): 체크포인트
- [`schemas/common.schema.json`](../schemas/common.schema.json): 공통 타입

모든 스키마는 JSON Schema Draft 2020-12를 사용한다. JSON은 검증 가능한 원본이고 Markdown은 사람이 읽는 파생 표현이다.

## 내부 제어 영역

`.work-harvest/`는 쓰기 잠금, 진행 중인 트랜잭션과 복구 격리를 위한 내부 디렉터리다. 업무 기록 원본이나 보고서 집계 대상이 아니며 애플리케이션을 통하지 않고 편집하지 않는다.

```text
.work-harvest/
  write.lock
  transactions/
  quarantine/
```

구체적인 잠금·충돌·복구 규약은 [ADR 0002](./adr/0002-recoverable-local-write-transactions.md)를 따른다.

## 식별자

- `project_id`: 제품 또는 코드베이스 경계
- `work_item_id`: 세션과 날짜를 넘어 유지되는 업무 항목
- `checkpoint.id`: 체크포인트마다 생성하는 고유 식별자
- `source.session_ref`: 에이전트 제품이 제공할 때만 저장하는 세션 참조

세션 참조를 얻지 못해도 체크포인트를 생성할 수 있어야 한다. 분류와 보고의 중심 키는 `work_item_id`다.

브랜치와 세션은 업무 항목의 식별자가 아니라 작업 위치와 근거다. 같은 결과물을 여러 브랜치 또는 세션에서 수행할 수 있고, 한 세션에서 여러 독립 업무 항목을 다룰 수도 있다.

`source.agent: manual`은 사용자가 별도 기록 세션에서 전달한 내용처럼 현재 에이전트 작업 밖에서 수행된 업무를 기록할 때 사용한다. 사용자 설명은 유효한 기록 출처지만 독립 검증을 의미하지 않는다. 확인되지 않은 결과는 설명이나 영향에 그 한계를 표시하고 후속 검증으로 남긴다.

## 업무 context

`context.json`은 현재 상태, 유효한 결정, 주요 파일, 검증, 다음 작업과 리스크를 저장한다. `context.md`는 이 데이터를 사람이 읽을 수 있게 렌더링한다. 체크포인트의 `context_update`에서 전달된 필드만 교체하고 전달되지 않은 필드는 유지한다.

## 업무 항목 생성·수정 계약

업무 항목 하나는 다음 세 파일을 하나의 변경 집합으로 취급한다.

```text
work-items/<id>/work-item.json
work-items/<id>/context.json
work-items/<id>/context.md
```

생성은 세 경로 모두에 create-only 규칙을 적용하고, 하나라도 이미 있으면 기존 파일을 덮어쓰지 않는다. 수정은 편집 시 읽은 세 파일 각각의 SHA-256 revision을 요구한다. 한 파일이라도 사라졌거나 바뀌었으면 세 파일 전체 수정을 거부한다.

수정 가능한 업무 필드는 제목, 상태, 목표, 기대 결과, 분류, 저장소와 링크다. `id`, `project_id`, `created_at`, `context_path`는 생성 후 바꾸지 않는다. 상태가 `completed`로 바뀔 때 `completed_at`을 명시하지 않으면 수정 시각을 사용하고, 완료가 아닌 상태로 바뀌면 `null`로 되돌린다. 수정할 때 업무 항목과 Context의 `updated_at`은 같은 값으로 갱신한다.

Context patch는 전달된 현재 상태, 결정, 파일, 검증, 다음 작업, 리스크와 Git 기준점만 교체한다. `context.md`는 저장된 두 JSON에서 항상 다시 생성하며 수동 편집본을 원본으로 사용하지 않는다. JSON은 2칸 들여쓰기와 마지막 개행을 사용하고 중첩 저장소·링크 객체의 필드 순서를 보존해 기존 Node CLI와 불필요한 diff가 생기지 않게 한다.

체크포인트 JSON도 같은 이름의 Markdown으로 렌더링한다.

```text
records/YYYY/MM/DD/<checkpoint_id>.json
records/YYYY/MM/DD/<checkpoint_id>.md
```

## 날짜

`captured_at`은 항상 정확한 생성 시각이다. 실제 작업 시점은 `work_period`가 표현한다. `precision`과 `basis`는 날짜 정보의 신뢰도를 명시한다.

뒤늦게 기록한 체크포인트는 과거의 `captured_at`을 만들지 않는다. 체크포인트 파일은 생성일 폴더에 저장하고 보고서는 `work_period`를 기준으로 기간을 판단한다.

## 템플릿

- [`templates/work-item-context.md`](../templates/work-item-context.md): 현재 업무 상태와 인수인계
- [`templates/checkpoint.md`](../templates/checkpoint.md): 사람이 읽는 체크포인트 기록
- [`templates/performance-note.md`](../templates/performance-note.md): 성과 노트 공통 템플릿

성과 노트는 `reports/performance-notes/` 아래 Markdown으로 생성한다. 이 문서는 체크포인트 원본을 보존하는 파생 초안이지만, 사용자가 정량 수치·배포 정보·공유 문구를 보완하는 최종 문서이므로 생성 뒤 수동 편집을 허용한다. 근거가 없는 내용은 자동으로 추측하지 않고 `미확인`으로 남긴다.

템플릿의 안내 문구는 실제 파일 생성 시 제거해야 한다. 빈 항목은 스키마에 맞는 빈 배열이나 `null`로 정규화한다.

## 검증 규칙

- 완료 상태의 업무 항목은 `completed_at`이 있어야 한다.
- 완료가 아닌 업무 항목의 `completed_at`은 `null`이어야 한다.
- 최종 체크포인트의 `status_after`는 `completed`여야 하며 확인된 결과가 하나 이상 있어야 한다.
- 정정 체크포인트는 `correction_of`를 가져야 한다.
- 일반 체크포인트는 `correction_of: null`이어야 한다.
- 체크포인트에는 활동, 결정, 검증, 결과, 차단 요소 또는 다음 작업 중 최소 하나가 있어야 한다.
- `precision: unknown`이면 작업 기간의 시작과 종료는 `null`이어야 한다.
- 체크포인트에는 원문 transcript나 명령 출력 전체를 기본적으로 저장하지 않는다.
