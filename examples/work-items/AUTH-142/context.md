---
schema_version: "1.0"
work_item_id: "AUTH-142"
project_id: "jajak-front"
title: "인증 시스템 개선"
status: "in_progress"
updated_at: "2026-07-13T18:10:00+09:00"
last_checkpoint_id: "CP-20260713-001"
last_verified_git_ref: "abc1234"
---

# AUTH-142 인증 시스템 개선

## 목표

토큰 만료 시 자동으로 갱신하고 실패한 요청을 안전하게 재시도한다.

## 현재 상태

refresh token 서비스와 interceptor 연동을 완료했다. 동시 요청 테스트를 작성하는 중이다.

## 주요 결정과 이유

- 동시에 발생한 갱신 요청은 하나의 Promise를 공유한다. 중복 refresh 요청을 막기 위해서다.
- 원 요청 재시도는 한 번만 허용한다. 무한 재시도를 방지하기 위해서다.

## 주요 파일과 문서

- `src/auth/refresh-token.ts`: refresh 요청과 동시성 제어
- `src/api/interceptor.ts`: 인증 실패 감지와 요청 재시도
- `tests/auth/refresh-token.test.ts`: 인증 갱신 테스트

## 검증 상태

- 완료: 기본 갱신 성공 테스트
- 미완료: 동시 요청과 refresh token 만료 테스트

## 남은 작업

- 동시 401 응답 테스트 작성
- refresh token 만료 UX 확인
- 전체 인증 테스트 실행

## 리스크와 확인할 사항

- 운영 API의 refresh token 만료 응답 규격을 확인해야 한다.

## 마지막으로 확인한 Git 기준점

- 저장소: `jajak-front`
- 브랜치: `feat/auth-refresh`
- 커밋: `abc1234`
- 확인 시각: `2026-07-13T18:10:00+09:00`
