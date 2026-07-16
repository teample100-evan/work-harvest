import { Check, Search } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState } from "react";

interface ProjectComboboxProps {
  disabled?: boolean;
  onChange: (value: string) => void;
  options: string[];
  placeholder?: string;
  value: string;
}

export function ProjectCombobox({
  disabled = false,
  onChange,
  options,
  placeholder = "프로젝트 이름 또는 ID",
  value,
}: ProjectComboboxProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const listboxId = useId();
  const filteredOptions = useMemo(() => {
    const query = value.trim().toLocaleLowerCase("ko-KR");
    if (!query) return options.slice(0, 8);
    return options
      .filter((option) => option.toLocaleLowerCase("ko-KR").includes(query))
      .slice(0, 8);
  }, [options, value]);

  useEffect(() => {
    if (!open) return;

    function closeOnOutsideClick(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    }

    document.addEventListener("pointerdown", closeOnOutsideClick);
    return () => document.removeEventListener("pointerdown", closeOnOutsideClick);
  }, [open]);

  return (
    <div className="project-combobox" ref={rootRef}>
      <div className="project-combobox-input">
        <Search aria-hidden="true" size={15} strokeWidth={1.8} />
        <input
          aria-autocomplete="list"
          aria-controls={listboxId}
          aria-expanded={open}
          aria-label="프로젝트"
          autoComplete="off"
          disabled={disabled}
          onChange={(event) => {
            onChange(event.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onKeyDown={(event) => {
            if (event.key === "Escape") setOpen(false);
          }}
          placeholder={placeholder}
          required
          role="combobox"
          value={value}
        />
      </div>
      {open && !disabled && filteredOptions.length > 0 && (
        <div className="project-combobox-options" id={listboxId} role="listbox">
          <span>기존 프로젝트</span>
          {filteredOptions.map((option) => (
            <button
              aria-selected={option === value}
              key={option}
              onClick={() => {
                onChange(option);
                setOpen(false);
              }}
              role="option"
              type="button"
            >
              <span>{option}</span>
              {option === value && <Check aria-hidden="true" size={14} strokeWidth={2} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
