import { Select } from "@base-ui/react/select";
import { Check, ChevronDown } from "lucide-react";

export interface SelectMenuOption<T extends string> {
  value: T;
  label: string;
}

interface SelectMenuProps<T extends string> {
  ariaLabel: string;
  className?: string;
  disabled?: boolean;
  onChange: (value: T) => void;
  options: ReadonlyArray<SelectMenuOption<T>>;
  value: T;
}

export function SelectMenu<T extends string>({
  ariaLabel,
  className = "",
  disabled = false,
  onChange,
  options,
  value,
}: SelectMenuProps<T>) {
  const selected = options.find((option) => option.value === value) ?? options[0];

  return (
    <div className={`select-menu ${className}`.trim()}>
      <Select.Root<T>
        disabled={disabled}
        items={options}
        modal={false}
        onValueChange={(nextValue) => {
          if (nextValue !== null) onChange(nextValue);
        }}
        value={value}
      >
        <Select.Trigger
          aria-label={`${ariaLabel}, 현재 ${selected?.label ?? value}`}
          className="select-menu-trigger"
        >
          <Select.Value className="select-menu-value" />
          <Select.Icon className="select-menu-icon">
            <ChevronDown aria-hidden="true" size={14} strokeWidth={1.8} />
          </Select.Icon>
        </Select.Trigger>
        <Select.Portal>
          <Select.Positioner
            align="end"
            alignItemWithTrigger={false}
            className="select-menu-positioner"
            collisionPadding={8}
            sideOffset={5}
          >
            <Select.Popup className="select-menu-options" aria-label={ariaLabel}>
              <Select.List>
                {options.map((option) => (
                  <Select.Item
                    className="select-menu-option"
                    key={option.value}
                    value={option.value}
                  >
                    <Select.ItemText>{option.label}</Select.ItemText>
                    <Select.ItemIndicator className="select-menu-option-indicator">
                      <Check aria-hidden="true" size={14} strokeWidth={2} />
                    </Select.ItemIndicator>
                  </Select.Item>
                ))}
              </Select.List>
            </Select.Popup>
          </Select.Positioner>
        </Select.Portal>
      </Select.Root>
    </div>
  );
}
