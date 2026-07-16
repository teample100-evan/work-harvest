type FormControl = HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;

function isFormControl(value: EventTarget | null): value is FormControl {
  return (
    value instanceof HTMLInputElement ||
    value instanceof HTMLSelectElement ||
    value instanceof HTMLTextAreaElement
  );
}

function controlLabel(control: FormControl) {
  const explicit = control.getAttribute("aria-label");
  if (explicit) return explicit;
  const label = control.closest("label");
  const text = label?.querySelector(":scope > span")?.textContent?.trim();
  return text || "필수 항목";
}

function validationMessage(control: FormControl) {
  const label = controlLabel(control);
  if (control.validity.valueMissing) return `${label}을(를) 입력해 주세요.`;
  if (control.validity.patternMismatch) return `${label} 형식을 확인해 주세요.`;
  if (control.validity.tooShort) return `${label}을(를) 조금 더 길게 입력해 주세요.`;
  if (control.validity.tooLong) return `${label}이(가) 허용 길이를 초과했습니다.`;
  if (control.validity.typeMismatch) return `${label}에 올바른 값을 입력해 주세요.`;
  return `${label} 입력 내용을 확인해 주세요.`;
}

export function clearControlValidation(target: EventTarget | null) {
  if (!isFormControl(target)) return;
  target.setCustomValidity("");
  target.removeAttribute("aria-invalid");
}

export function validateControls(root: ParentNode) {
  const controls = root.querySelectorAll<FormControl>("input, select, textarea");
  for (const control of controls) {
    control.setCustomValidity("");
    control.removeAttribute("aria-invalid");
    if (control.checkValidity()) continue;
    const message = validationMessage(control);
    control.setCustomValidity(message);
    control.setAttribute("aria-invalid", "true");
    control.focus();
    control.reportValidity();
    return message;
  }
  return null;
}
