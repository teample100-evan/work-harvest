# 데스크톱 타이포그래피 상향 감사

## 범위

- Pretendard 기반 기본 UI 크기를 16px로 상향
- 목록·상세·새 업무 화면의 넓은 창과 축소 창 비교

## 확인 단계

1. `01-list-before.png` — 상향 전 목록 기준 화면
2. `02-list-after.png` — 상향 후 목록 가독성 확인, 통과
3. `07-list-compact-after.png` — 축소 목록 정렬과 잘림 확인, 통과
4. `08-detail-compact-after.png` — 18px 상세 본문 재배치 확인, 통과
5. `09-create-compact-after.png` — 축소 생성 화면의 필드·카드 배치 확인, 통과
6. `10-create-min-width-after.png` — 최소 폭 2열 시나리오와 푸터 정렬 확인, 통과

## 결과

- 기본 UI 16px, 상세 본문 18px, 주요 제목 28–30px 체계 적용
- 보조 정보는 12–14px로 유지해 정보 계층을 보존
- 확대된 글꼴에 맞춰 사이드바·컨트롤 높이·본문 간격을 함께 조정

final result: passed
