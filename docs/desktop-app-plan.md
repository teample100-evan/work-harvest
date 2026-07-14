# Work Harvest 데스크톱 앱 구현 계획

- 상태: M3 업무 항목 쓰기 Core 완료, M2 하루 soak 병행
- 최초 작성일: 2026-07-14
- 대상: macOS Apple Silicon
- 관련 결정: [ADR 0001](./adr/0001-tauri-desktop-app.md), [ADR 0002](./adr/0002-recoverable-local-write-transactions.md)

## 1. 목표

Work Harvest의 로컬 파일 데이터를 안전하게 탐색하고, Codex나 CLI가 만든 변경을 실행 중인 앱에 자동 반영하는 Tauri 데스크톱 앱을 만든다.

첫 번째 제품 목표는 읽기 전용 내부 알파다. 쓰기 기능과 기존 CLI 교체는 읽기 흐름과 상시 실행 가치가 확인된 뒤 진행한다.

## 2. 성공 기준

읽기 전용 내부 알파는 다음 조건을 만족해야 한다.

- 사용자가 데이터 루트 폴더를 선택할 수 있다.
- 선택한 데이터 루트가 앱 재실행 후 복구된다.
- 업무 항목, context와 체크포인트 수를 확인할 수 있다.
- 기본 데이터 구조 문제를 앱에서 확인할 수 있다.
- Codex나 CLI가 파일을 추가·수정하면 앱이 자동으로 다시 검사한다.
- 잘못된 JSON 하나 때문에 앱 전체가 종료되지 않는다.
- 기존 Node.js CLI와 테스트가 그대로 통과한다.
- Apple Silicon 대상 앱 빌드를 만들 수 있다.

초기 성능 목표는 측정값을 확보하기 위한 기준이며 출시 보장은 아니다.

| 항목 | 목표 |
| --- | --- |
| 개발 빌드가 아닌 배포 앱 크기 | 20MB 이하 |
| 유휴 상태 메모리 | 100MB 이하 권장, 150MB 재검토선 |
| 외부 파일 변경 반영 | 1초 이내 |
| 1,000개 업무·10,000개 체크포인트 초기 표시 | 2초 이내 |

## 3. 범위

### 초기 포함

- Tauri v2 앱 골격
- React·TypeScript 프론트엔드
- 데이터 루트 선택과 복구
- Rust 읽기 전용 데이터 검사
- 파일 변경 감시와 UI 갱신
- 업무 현황 대시보드
- 검증 문제 표시
- Apple Silicon 빌드 측정

### 초기 제외

- 업무 항목 및 체크포인트 편집
- 성과 노트 생성
- 메뉴바 상세 팝오버와 알림 정책
- Windows·Linux·모바일
- App Store 배포
- 계정, 클라우드 동기화와 팀 공유
- 주간·월간 보고서
- 데이터베이스

## 4. 목표 저장소 구조

```text
apps/
  desktop/
    src/                    React·TypeScript UI
    src-tauri/              Tauri 애플리케이션 계층
crates/
  work-harvest-core/        공유 Rust 도메인 라이브러리
  work-harvest-cli/         장기 Rust CLI
src/                        전환 기간의 기존 Node.js 구현
schemas/                    공통 JSON Schema
templates/                  공통 Markdown 템플릿
```

## 5. 마일스톤

### M0. 기반과 기술 검증

상태: 완료

- ADR과 구현 계획 작성
- pnpm·Cargo workspace 구성
- Tauri, React와 TypeScript 앱 생성
- 데이터 루트 선택
- 기본 Rust 데이터 검사
- 외부 파일 변경 감시
- 프론트엔드 자동 갱신
- 앱 크기와 메모리 기준선 측정

완료 조건:

- `pnpm desktop:build`가 성공한다.
- `cargo test --workspace`가 성공한다.
- `pnpm check`로 기존 CLI 검증이 성공한다.
- 로컬 데이터 루트 선택과 파일 변경 반영을 수동 확인한다.

검증 결과(2026-07-14):

| 항목 | 결과 |
| --- | --- |
| 기존 Node CLI | 통합 테스트 4개와 예제 검증 통과 |
| Rust Core·Tauri | Rust 테스트 3개와 workspace 빌드 통과 |
| macOS 산출물 | 앱 11.3MB, DMG 3.0MB |
| 실행 중 메모리 | WebKit 보조 프로세스를 포함한 physical footprint 약 118MB |
| 실제 데이터 | `~/work-records`의 업무 5개, context 5개, 체크포인트 12개 검사 |
| 외부 변경 감지 | 파일 추가·삭제가 debounce 후 1초 이내 화면에 반영됨 |

M0 검사는 JSON 파싱, 필수 필드, 중복 ID와 파일 간 기본 관계를 확인한다. 공통 JSON Schema 전체 검증은 M1 범위다.

### M1. 읽기 전용 내부 알파

상태: 완료

완료:

- 업무 항목 검색과 상태 필터
- 업무 목표, 분류와 현재 context 상세
- 체크포인트 타임라인과 검증 요약
- 공통 스키마를 내장한 Draft 2020-12 전체 JSON Schema 검증
- context와 업무 항목 간 ID·프로젝트 관계 검사
- 필터 결과와 선택 업무 상세 자동 동기화
- 체크포인트 활동·결과·근거 전체 상세
- Finder에서 업무 폴더 표시
- Context와 체크포인트를 기본 Markdown 앱으로 열기
- 빈 상태, 손상 데이터와 권한 오류별 복구 안내

완료 조건:

- 예제와 실제 데이터 루트에서 동일한 탐색 흐름이 동작한다.
- 각 오류가 해당 파일 경로와 함께 표시된다.
- 외부 체크포인트가 1초 이내에 화면에 반영된다.

1차 구현 검증 결과(2026-07-14):

| 항목 | 결과 |
| --- | --- |
| Rust Core | 상세 조회·스키마 위반·안전 경로 해석을 포함한 테스트 7개 통과 |
| 실제 데이터 | 업무 5개와 체크포인트 12개 전체 스키마 통과 |
| 검색·상세 UI | 검색 결과 선택 동기화와 여러 업무 상세 전환 수동 확인 |
| 체크포인트 상세 | 활동·결정·검증·결과·근거·Git 기준점 펼침 확인 |
| 외부 앱 연결 | Finder 업무 선택, Context·체크포인트 Markdown 열기 확인 |
| macOS 산출물 | 앱 15.9MB, DMG 4.5MB |
| 실행 중 메모리 | WebKit 보조 프로세스 포함 초기 약 101MB, 상세·외부 앱 QA 후 약 124MB |
| 메모리 최적화 | 다중 배경 블러 제거로 약 157MB에서 약 101MB로 감소 |

### M2. 상시 실행 경험

상태: 진행 중 — 상시 실행·증분 인덱스 완료

완료:

- 창을 닫아도 메뉴바에서 실행 유지
- 메뉴바에서 최근 업무 5개와 마지막 기록 표시
- 메뉴 업무 선택 시 해당 업무를 선택한 창 복원
- 사용자가 명시적으로 켜는 새 체크포인트와 새 검증 오류 알림
- 스냅샷 기준선으로 동일 사실 중복 알림 방지
- 창 위치와 크기 저장·복구
- JSON 문서를 메모리에 유지하는 데이터 루트 인덱스
- 변경 경로에서 영향받은 업무 ID를 계산하는 증분 갱신
- 350ms 유휴·1초 최대 지연 배치와 중복 경로 제거
- 선택한 업무가 영향받은 경우에만 상세 데이터 다시 읽기
- 대량 이벤트 압축 검사와 실제 macOS watcher soak 하네스

남음:

- 실제 24시간 watcher soak 실행과 결과 기록

완료 조건:

- 하루 동안 실행한 상태에서 파일 감시 누락과 이벤트 폭주가 없다.
- 알림은 새 사실이 있을 때만 한 번 표시된다.

1차 구현 검증 결과(2026-07-14):

| 항목 | 결과 |
| --- | --- |
| 메뉴바 상주 | 창 닫기 후 창은 숨겨지고 앱 프로세스와 파일 감시는 유지됨 |
| 메뉴바 내용 | 데이터 루트 검사 시 최근 업무 5개·마지막 체크포인트·수량 메뉴 갱신 |
| 창 상태 | 2240×1520 크기와 위치가 `.window-state.json`에 저장되고 재실행 시 복구됨 |
| 신규 기록 감지 | 임시 데이터에서 체크포인트 생성 후 1초 이내 1개→2개 자동 반영 |
| 알림 | 앱 내부 opt-in 전에는 미발송, 첫 발송에서 macOS 권한 요청 표시 확인 |
| 중복 방지 | 체크포인트 ID와 오류 지문 스냅샷 차이만 알림 대상으로 사용 |
| 자동 검증 | Node 통합 테스트 4개, Rust Core 7개·Desktop 3개, 프론트엔드 빌드와 Clippy 통과 |
| macOS 산출물 | 앱 16.7MB, DMG 4.8MB |
| 상주 메모리 | WebKit 보조 프로세스 포함 physical footprint 약 100MB |

증분 인덱스 구현 검증 결과(2026-07-14):

| 항목 | 결과 |
| --- | --- |
| Core 증분 정확성 | 변경 JSON 1개만 다시 읽고 영향 업무 ID를 반환하며 전체 재검사 결과와 일치 |
| 관계 파일 반영 | JSON 재로딩 없이 context·체크포인트 Markdown 누락 상태 갱신 |
| 반복 수렴 | context 250회 연속 변경 후 캐시 스냅샷과 전체 검사 결과 일치 |
| 이벤트 폭주 | 합성 이벤트 10,000건을 32개 고유 경로로 압축 |
| 실제 watcher 예행 | macOS FSEvents로 5초 연속 기록 변경 후 최종 전체 검사와 일치 |
| 자동 검증 | Rust Core 11개, Desktop 4개 통과·24시간 soak 1개 opt-in, 프론트엔드 빌드 통과 |

24시간 검증은 다음 opt-in 테스트로 실행한다.

```sh
WORK_HARVEST_SOAK_SECONDS=86400 cargo test -p work-harvest-desktop \
  watcher_soak_converges_to_the_full_scan -- --ignored --nocapture
```

하루가 실제로 경과한 검증 결과가 아직 없으므로 M2 전체를 완료 처리하지 않는다.

### M3. 안전한 쓰기 Core

상태: 진행 중 — 안전 쓰기 기반과 업무 항목 API 완료

완료:

- 데이터 루트 단일 writer advisory lock
- SHA-256 revision 기반 외부 변경 충돌 감지
- create-only 무덮어쓰기와 symlink·루트 탈출 거부
- 루트 내부 staged·backup·manifest 트랜잭션
- 부분 적용·post-write 검증 실패 전체 rollback
- 재시작 시 미완료 트랜잭션 자동 복구
- 안전하게 복구할 수 없는 변경 quarantine과 후속 쓰기 차단
- Node CLI와 같은 업무 항목·Context 정규화와 `context.md` 렌더링
- 세 파일을 함께 생성하는 create-only 업무 항목 API
- 세 파일의 SHA-256 revision을 함께 반환하는 편집 snapshot API
- 불변 식별자를 보존하고 세 파일을 함께 교체하는 업무 항목·Context patch API
- 변경 업무의 스키마·상호 참조·파생 Markdown post-write 검증
- `repositories`와 `links` 내부 JSON 필드 순서 보존

남음:

- 체크포인트 작성
- 체크포인트 작성에 따른 context 갱신
- 성과 노트 생성
- 저장 전 diff와 검증 표시
- 기존 Node CLI의 잠금 호환 또는 Rust CLI writer 전환

안전 쓰기 기반 검증 결과(2026-07-14):

| 항목 | 결과 |
| --- | --- |
| 단일 writer | 같은 데이터 루트의 두 번째 writer가 즉시 `LockBusy`로 중단되고 첫 writer 종료 후 재획득 |
| 무덮어쓰기·충돌 | 기존 create 대상과 stale SHA-256 교체를 거부하고 원본 보존 |
| 부분 실패 rollback | 두 번째 연산 전 강제 실패와 post-write 검증 실패 모두 첫 파일까지 원상복구 |
| 재시작 복구 | `applying`은 rollback하고 `committed`는 최종 대상을 유지한 채 임시 자산 정리 |
| 보수적 격리 | 복구 대상이 다시 변경되면 원본을 건드리지 않고 quarantine 후 쓰기 차단 |
| 경로 안전성 | 루트 탈출·내부 제어 경로·symlink 경유 쓰기 거부 |
| 자동 검증 | Rust Core 24개·Desktop 4개·Node 통합 4개, 프론트엔드 빌드와 Clippy 통과 |

업무 항목 쓰기 API 검증 결과(2026-07-14):

| 항목 | 결과 |
| --- | --- |
| Node 호환 | 기본값·완료 상태와 중첩 객체를 포함한 두 입력에서 `work-item.json`·`context.json`·`context.md`가 바이트 단위로 일치 |
| 생성 원자성 | JSON 두 개와 Markdown 하나를 create-only 단일 트랜잭션으로 생성하고 중복 생성 시 기존 세 파일 보존 |
| 수정 일관성 | `id`·`project_id`·`created_at`·`context_path`를 보존하고 업무·Context·Markdown을 한 revision 집합으로 교체 |
| 충돌 보호 | 세 파일 중 하나라도 편집 후 바뀌면 나머지 두 파일을 건드리지 않고 전체 수정 거부 |
| 검증 보호 | 잘못된 status 등 스키마 위반 patch를 파일 교체 전에 거부하고 저장 후 파생 Markdown 재현성 검사 |
| 자동 검증 | Rust Core 30개와 Core 전체 Clippy 통과 |

완료 조건:

- [x] GUI Core와 기존 Node CLI의 정규화 결과가 호환된다.
- [ ] 모든 사용자 노출 writer가 같은 advisory lock에 참여해 동시 쓰기에서 기존 기록을 덮어쓰지 않는다.
- [x] 실패 후 변경 집합이 검증 가능한 상태로 복구된다.

업무 항목 Core API는 준비됐지만 기존 Node CLI가 advisory lock에 아직 참여하지 않는다. 따라서 실제 GUI의 생성·수정 명령은 잠금 호환 또는 Rust CLI 전환 뒤에 연결한다.

### M4. Rust CLI 통합

상태: 예정

- `work-harvest-core`를 사용하는 Rust CLI 구현
- 기존 명령과 JSON 출력 호환
- 기존 fixture를 이용한 Node·Rust 비교 테스트
- Codex Skill 래퍼 연결
- 검증 후 Node.js 구현 단계적 제거

완료 조건:

- 기존 통합 테스트에 대응하는 Rust 테스트가 통과한다.
- Codex Skill의 사용 명령과 사용자 데이터 형식이 바뀌지 않는다.

### M5. 직접 배포

상태: 예정

- 앱 아이콘과 번들 식별자 확정
- Developer ID 서명과 Apple 공증
- DMG 생성
- 서명된 자동 업데이트
- GitHub Releases 배포
- 필요하면 Universal binary 추가

## 6. 데이터 무결성 전략

읽기 전용 단계는 파일을 수정하지 않는다. 쓰기 기능은 다음 안전장치를 모두 구현한 뒤 활성화한다.

1. 데이터 루트마다 하나의 쓰기 잠금을 사용한다.
2. 기존 체크포인트는 exclusive create로만 생성한다.
3. 관련 파일은 임시 트랜잭션 디렉터리에서 준비한다.
4. 모든 산출물을 검증한 뒤 rename으로 반영한다.
5. 변경 전 파일 해시와 현재 해시가 다르면 충돌로 중단한다.
6. 중단된 트랜잭션은 다음 실행에서 감지하고 복구 또는 격리한다.
7. 저장 후 이번 변경 집합의 스키마·관계·파생 파일 재현성을 검사한다. 기존의 무관한 오류는 새 변경의 commit을 막지 않고 별도 루트 검사에서 계속 표시한다.

## 7. 호환성 테스트 전략

- `examples/`를 Rust 구현의 golden fixture로 사용한다.
- 기존 Node 통합 테스트와 같은 임시 데이터 루트 시나리오를 Rust에서도 실행한다.
- 시간과 자동 생성 ID는 테스트에서 주입 가능하게 만든다.
- JSON은 의미 구조를 비교하고 Markdown은 snapshot으로 비교한다.
- 업무 항목 정규화 전환기에는 Node와 Rust가 만든 canonical JSON·Markdown 바이트도 직접 비교한다.
- 오류 종류, 종료 코드와 주요 메시지를 비교한다.
- Node 구현 제거 전 실제 사용자 데이터 복사본으로 read-only 검증을 수행한다.

## 8. 위험과 대응

| 위험 | 대응 |
| --- | --- |
| Ajv와 Rust JSON Schema 구현 차이 | 공통 invalid fixture와 관계 검증 테스트 유지 |
| Codex와 GUI의 동시 쓰기 | 쓰기 기능 전에 잠금과 트랜잭션 구현 |
| macOS 폴더 접근 권한 손실 | 사용자가 선택한 루트만 사용하고 접근 복구 흐름 제공 |
| 대량 파일 감시 이벤트 폭주 | 350ms 유휴·1초 최대 배치, 경로 중복 제거와 영향 업무 단위 갱신 |
| Rust 이전이 기능 개발을 지연 | 읽기부터 단계적으로 이전하고 Node CLI를 유지 |
| UI 범위 확장 | 각 마일스톤의 제외 범위를 완료 전 변경하지 않음 |

## 9. 첫 구현 묶음

첫 변경은 다음 기능만 포함한다.

1. 계획과 ADR
2. workspace와 Tauri 앱 골격
3. 데이터 루트 폴더 선택
4. 업무 항목·context·체크포인트 개수 검사
5. 발견한 JSON 읽기 오류 표시
6. 외부 파일 변경 시 자동 재검사
7. 프론트엔드·Rust·기존 Node 테스트

업무 상세, 전체 JSON Schema 검증, 메뉴바와 쓰기 기능은 다음 변경으로 분리한다.

## 10. 변경 기록

| 날짜 | 변경 |
| --- | --- |
| 2026-07-14 | Tauri 선택, 단계적 Rust Core 이전과 읽기 전용 MVP 계획 확정 |
| 2026-07-14 | M0 구현·실데이터 수동 검증·Apple Silicon 앱과 DMG 빌드 완료 |
| 2026-07-14 | M1 1차 업무 상세·검색·타임라인·전체 JSON Schema 검증 구현 |
| 2026-07-14 | M1 체크포인트 전체 상세·Finder·Markdown·오류 복구 흐름 완료 |
| 2026-07-14 | M2 메뉴바 상주·최근 업무·알림 opt-in·창 상태 복구 1차 구현 완료 |
| 2026-07-14 | M2 캐시형 증분 인덱스·업무 단위 갱신·watcher soak 하네스 구현 |
| 2026-07-14 | M3 단일 writer 잠금·revision 충돌·복구 가능한 다중 파일 트랜잭션 기반 구현 |
| 2026-07-14 | M3 Node 호환 업무 항목 생성·revision 보호 수정·편집 snapshot Core API 구현 |
