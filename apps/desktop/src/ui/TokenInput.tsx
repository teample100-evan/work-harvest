import { X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";

interface TokenInputProps {
  ariaLabel: string;
  onChange: (value: string) => void;
  placeholder?: string;
  value: string;
}

function parseTokens(value: string): string[] {
  return value
    .split(",")
    .map((token) => token.trim())
    .filter(Boolean);
}

export function TokenInput({ ariaLabel, onChange, placeholder, value }: TokenInputProps) {
  const [input, setInput] = useState("");
  const tokens = parseTokens(value);

  function commitInput() {
    const nextTokens = parseTokens(input);
    if (nextTokens.length === 0) return;
    onChange(Array.from(new Set([...tokens, ...nextTokens])).join(", "));
    setInput("");
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter" || event.key === ",") {
      event.preventDefault();
      commitInput();
      return;
    }
    if (event.key === "Backspace" && !input && tokens.length > 0) {
      onChange(tokens.slice(0, -1).join(", "));
    }
  }

  return (
    <div className="token-input">
      {tokens.map((token) => (
        <span className="token-input-chip" key={token}>
          {token}
          <button
            aria-label={`${token} 삭제`}
            onClick={(event) => {
              event.stopPropagation();
              onChange(tokens.filter((current) => current !== token).join(", "));
            }}
            type="button"
          >
            <X aria-hidden="true" size={11} strokeWidth={2} />
          </button>
        </span>
      ))}
      <input
        aria-label={ariaLabel}
        onBlur={commitInput}
        onChange={(event) => setInput(event.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={tokens.length === 0 ? placeholder : "추가 입력"}
        value={input}
      />
    </div>
  );
}
