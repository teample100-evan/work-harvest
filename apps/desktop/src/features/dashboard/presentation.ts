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
