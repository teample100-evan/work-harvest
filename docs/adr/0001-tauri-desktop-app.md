# ADR 0001: Tauri 기반 macOS 데스크톱 앱

- 상태: 승인
- 결정일: 2026-07-14
- 적용 범위: Work Harvest 데스크톱 앱과 장기 CLI 구조

## 배경

Work Harvest는 JSON을 원본으로, Markdown을 파생 문서로 사용하는 로컬 파일 기반 업무 기록 도구다. 현재 Node.js CLI와 Codex Skill로 업무 항목, 체크포인트, 인수인계 context와 성과 노트를 관리한다.

사용자는 작업 중 앱을 계속 실행해 두고 다음 내용을 반복해서 확인할 가능성이 높다.

- 진행 중인 업무와 다음 단계
- 마지막 체크포인트 이후의 경과
- Codex나 CLI가 새로 만든 기록
- 데이터 검증 오류
- 업무별 체크포인트 타임라인

따라서 데스크톱 앱은 일회성 편집기보다 파일 변경을 감시하는 상시 실행 로컬 허브여야 한다.

## 결정

Work Harvest 데스크톱 앱은 Tauri v2와 React·TypeScript로 구현한다.

초기 대상은 macOS Apple Silicon이며 직접 배포하는 앱과 DMG를 우선한다. App Store, Windows, Linux와 모바일은 초기 범위에서 제외한다.

최종 구조에서는 Node.js sidecar를 포함하지 않는다. Rust 라이브러리 `work-harvest-core`를 데이터 접근과 도메인 규칙의 단일 구현으로 사용하고, 장기적으로 Tauri 앱과 Rust CLI가 이 라이브러리를 공유한다.

기존 Node.js CLI는 Rust 구현이 동작 호환성을 확보할 때까지 유지한다. 전환 기간에는 다음 원칙을 지킨다.

1. 읽기 전용 기능부터 Rust로 구현한다.
2. JSON Schema와 기존 예제·통합 테스트를 호환성 기준으로 사용한다.
3. 쓰기 기능은 잠금, 원자적 저장과 충돌 감지를 구현한 뒤 활성화한다.
4. 기존 Node CLI 제거는 Rust CLI의 출력과 실패 동작이 검증된 뒤 별도 결정한다.
5. Codex Skill의 `wh` 호출 인터페이스는 유지한다.

## 제품 원칙

- JSON은 검증 가능한 원본이다.
- Markdown은 재생성 가능한 사람이 읽는 표현이다.
- 체크포인트는 append-only다.
- GUI가 파일을 직접 수정하지 않고 Rust Core의 도메인 명령을 사용한다.
- GUI가 종료돼도 CLI와 Codex Skill은 독립적으로 동작한다.
- 외부 파일 변경은 앱 재시작 없이 반영한다.
- 로컬 데이터는 명시적인 사용자 동작 없이 외부로 전송하지 않는다.

## 초기 아키텍처

```text
React/TypeScript UI
        |
        | typed Tauri commands and events
        v
Tauri Rust application
        |
        v
work-harvest-core
        |
        +-- work-items/**/*.json
        +-- records/**/*.json
        +-- derived Markdown
```

프론트엔드는 임의의 파일 시스템 경로나 shell 명령을 받지 않는다. 다음처럼 목적이 한정된 명령만 호출한다.

- 데이터 루트 설정과 조회
- 데이터 루트 검사
- 업무 항목 목록과 상세 조회
- 체크포인트 목록과 상세 조회
- Finder 또는 기본 앱으로 원본 열기

## 고려한 대안

### Electron

현재 Node.js 구현을 직접 재사용할 수 있어 초기 개발 속도가 빠르다. 그러나 상시 실행 앱에서 Chromium과 Node.js 런타임의 설치 크기와 기본 자원 사용량이 제품 방향과 맞지 않는다고 판단했다.

### Tauri와 Node.js sidecar

기존 CLI를 빠르게 재사용할 수 있지만 Node.js 런타임을 앱에 포함하면 Tauri의 크기 이점이 크게 줄고 프로세스와 배포 구조가 복잡해진다. 최종 구조로 사용하지 않는다.

### SwiftUI 네이티브 앱

macOS 통합과 자원 효율은 가장 좋지만 기존 웹 기술 활용과 향후 플랫폼 확장 비용이 커진다. 현재 팀과 프로젝트 규모에서는 Tauri가 더 균형 잡힌 선택이다.

## 결과와 비용

### 기대 효과

- 시스템 WebView를 활용한 작은 설치 크기
- 메뉴바와 파일 감시를 포함한 상시 실행 경험
- Rust Core를 통한 CLI와 GUI 규칙 통합
- 로컬 파일 형식과 외부 편집 가능성 유지

### 수반되는 비용

- Rust와 Cargo 도구 체계 추가
- 기존 Node.js 동작을 Rust로 점진적으로 이전
- macOS 파일 접근, 서명과 공증 관리
- Node와 Rust 구현의 전환 기간 동안 호환성 테스트 유지

## 재검토 조건

다음 중 하나가 확인되면 이 결정을 재검토한다.

- 읽기 전용 프로토타입의 유휴 메모리가 150MB를 지속적으로 넘는다.
- macOS 파일 접근 제약 때문에 선택한 데이터 루트를 안정적으로 감시할 수 없다.
- Rust Core 이전 비용이 예측 범위를 크게 넘고 기능 개발을 장기간 정지시킨다.
- 실제 사용에서 상시 실행 또는 실시간 반영의 가치가 확인되지 않는다.
