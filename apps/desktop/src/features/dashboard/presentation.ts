export function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function friendlyError(error: unknown) {
  const message = errorMessage(error);
  if (/permission denied|operation not permitted/i.test(message)) {
    return "폴더 또는 파일 접근 권한이 없습니다. Finder에서 권한을 확인하거나 데이터 폴더를 다시 선택하세요.";
  }
  if (/not found|does not exist/i.test(message)) {
    return "연결된 파일을 찾을 수 없습니다. 외부에서 이동하거나 삭제했는지 확인한 뒤 다시 검사하세요.";
  }
  if (/could not open|could not reveal/i.test(message)) {
    return "Finder 또는 기본 Markdown 앱으로 파일을 열지 못했습니다. 연결 프로그램 설정을 확인하세요.";
  }
  return message;
}

export function issueGuidance(code: string) {
  if (code === "schema_validation") {
    return "해당 JSON 필드를 공통 스키마에 맞게 수정한 뒤 다시 검사하세요.";
  }
  if (code === "invalid_json") {
    return "JSON 문법 오류를 수정하면 파일 감지가 자동으로 다시 검사합니다.";
  }
  if (code === "read_failed" || code === "scan_failed") {
    return "Finder에서 파일 권한을 확인하거나 데이터 폴더를 다시 선택하세요.";
  }
  if (code.startsWith("missing_")) {
    return "원본 JSON과 파생 Markdown 생성 상태를 확인하세요.";
  }
  if (code.includes("mismatch") || code.startsWith("unknown_")) {
    return "업무·프로젝트·체크포인트 ID의 연결 관계를 확인하세요.";
  }
  return "파일 경로를 확인하고 원본 기록을 수정한 뒤 다시 검사하세요.";
}

export function formatTimestamp(value: string) {
  if (!value) return "기록 없음";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("ko-KR", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

const workItemStatusLabels: Record<string, string> = {
  planned: "예정",
  in_progress: "진행 중",
  blocked: "막힘",
  completed: "완료",
  cancelled: "취소됨",
};

const checkpointKindLabels: Record<string, string> = {
  started: "시작",
  progress: "진행",
  final: "완료",
  correction: "정정",
  backfill: "사후 기록",
};

const verificationKindLabels: Record<string, string> = {
  test: "테스트",
  build: "빌드",
  lint: "Lint",
  manual: "수동 확인",
  measurement: "측정",
  review: "리뷰",
  other: "기타",
};

const verificationStatusLabels: Record<string, string> = {
  passed: "통과",
  failed: "실패",
  partial: "부분 통과",
  not_run: "미실행",
};

const decisionStatusLabels: Record<string, string> = {
  accepted: "채택",
  proposed: "제안",
  superseded: "대체됨",
};

const confidentialityLabels: Record<string, string> = {
  normal: "일반",
  sensitive: "민감",
  restricted: "제한",
};

export function formatWorkItemStatus(status: string) {
  return workItemStatusLabels[status] ?? status;
}

export function needsWorkItemStatusBadge(status: string) {
  return status === "in_progress" || status === "blocked";
}

export function formatCheckpointKind(kind: string) {
  return checkpointKindLabels[kind.toLowerCase()] ?? kind;
}

export function formatVerificationKind(kind: string) {
  return verificationKindLabels[kind.toLowerCase()] ?? kind;
}

export function formatVerificationStatus(status: string) {
  return verificationStatusLabels[status.toLowerCase()] ?? status;
}

export function formatDecisionStatus(status: string) {
  return decisionStatusLabels[status.toLowerCase()] ?? status;
}

export function formatConfidentiality(value: string) {
  return confidentialityLabels[value.toLowerCase()] ?? value;
}
