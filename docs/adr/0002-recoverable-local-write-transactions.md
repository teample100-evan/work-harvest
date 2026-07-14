# ADR 0002: 복구 가능한 로컬 쓰기 트랜잭션

- 상태: 승인
- 날짜: 2026-07-14
- 범위: Rust Core와 데스크톱 쓰기 기능

## 배경

체크포인트 작성은 새 JSON·Markdown 생성과 업무 항목·Context 교체를 함께 수행한다. 이 파일들을 순서대로 직접 쓰면 프로세스 종료, 디스크 오류 또는 외부 편집 충돌이 발생했을 때 일부 파일만 새 상태가 될 수 있다.

로컬 파일 시스템은 여러 경로를 하나의 원자적 연산으로 교체하는 범용 트랜잭션을 제공하지 않는다. 따라서 성공한 쓰기는 모두 반영되고, 성공으로 확정되지 않은 쓰기는 다음 실행에서 원래 상태로 복구할 수 있어야 한다.

## 결정

### 단일 writer

데이터 루트의 `.work-harvest/write.lock`에 OS advisory exclusive lock을 건다. 잠금을 즉시 얻지 못하면 기다리거나 덮어쓰지 않고 `LockBusy`로 중단한다. 프로세스가 종료되면 OS가 잠금을 해제하므로 stale lock 파일을 임의로 삭제하지 않는다.

이 잠금은 advisory이므로 Work Harvest가 제공하는 모든 writer가 같은 규약에 참여해야 한다. 기존 Node CLI는 아직 이 잠금을 사용하지 않으므로 Rust 쓰기 명령을 사용자에게 노출하기 전에 호환 잠금 또는 Rust CLI 전환을 완료한다.

### revision 충돌 감지

기존 파일을 읽을 때 SHA-256과 byte 길이를 revision으로 얻는다. 교체 트랜잭션은 예상 SHA-256을 받아 준비 시점과 실제 적용 직전에 다시 비교한다. 파일이 사라지거나 내용이 달라지면 어떤 대상도 확정하지 않고 충돌로 반환한다.

새 파일은 create-only 연산이다. staged 파일과 대상이 같은 데이터 루트에 있으므로 hard link를 사용해 기존 경로를 덮어쓰지 않고 설치한다.

### 트랜잭션 상태

각 쓰기는 `.work-harvest/transactions/<transaction-id>/` 아래에 다음 자산을 만든다.

```text
manifest.json
staged/0000
backup/0000
```

manifest 상태는 다음 순서로만 이동한다.

1. `prepared`: 모든 새 내용을 기록하고 fsync했지만 원본은 건드리지 않았다.
2. `applying`: 기존 파일을 backup으로 이동하고 staged 파일을 대상에 설치한다.
3. `committed`: 모든 설치와 호출자 검증이 성공했다.

도메인 계층은 JSON Schema와 관계 검사를 통과한 내용만 트랜잭션에 전달한다. 모든 대상 설치 후 `committed`로 전환하기 전 데이터 루트 검증 callback을 한 번 더 실행하며, 실패하면 backup으로 rollback한다.

도메인 callback은 이번에 변경한 집합의 스키마, 상호 참조와 파생 파일 재현성을 검사한다. 데이터 루트에 이미 존재하던 무관한 오류 때문에 안전한 새 변경까지 막지는 않으며, 그런 오류는 별도의 전체 루트 검사에서 계속 사용자에게 표시한다.

### 복구와 격리

다음 writer가 잠금을 얻을 때 미완료 트랜잭션을 먼저 처리한다.

- `prepared`: 원본 변경이 없으므로 staged 디렉터리를 정리한다.
- `applying`: 새 대상의 hash가 manifest와 일치할 때만 제거하고 backup을 복원한다.
- `committed`: 대상은 확정됐으므로 남은 임시 자산만 정리한다.
- 복구 대상이 manifest와 다른 내용으로 다시 변경됨: 사용자의 변경을 덮어쓰지 않고 `.work-harvest/quarantine/`으로 옮긴 뒤 모든 후속 쓰기를 중단한다.

상대 경로의 `..`, 절대 경로, 내부 제어 디렉터리와 symlink 경유 경로는 거부한다.

## 결과

- 다중 파일 쓰기는 단일 파일 rename처럼 순간적으로 원자적이지 않지만 실패 후 원상복구할 수 있다.
- 파일 watcher가 applying 중간 상태를 관찰할 수 있다. 데스크톱의 350ms 배치가 일반적인 짧은 적용 구간을 합치며, 최종 UI 상태는 commit 또는 rollback 이후 스냅샷으로 수렴한다.
- quarantine은 자동 추측보다 데이터 보존을 우선한다. 향후 GUI에서 격리 상태와 복구 선택지를 표시해야 한다.
- `.work-harvest/`는 사용자 기록이 아니라 내부 제어 영역이며 데이터 인덱스와 보고서 대상에서 제외한다.

## 검증

- 동시 writer 잠금 경합
- create-only 무덮어쓰기
- stale SHA-256 충돌
- 두 번째 연산 전 실패 후 전체 rollback
- post-write 검증 실패 후 전체 rollback
- 중단된 `applying` 트랜잭션의 다음 실행 복구
- 재시작 시 `committed` 대상 유지와 임시 자산 정리
- 복구 중 외부 변경 발견 시 quarantine
- 손상된 manifest의 quarantine과 후속 쓰기 차단
- 루트 탈출과 symlink 경로 거부
