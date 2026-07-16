import { Dialog } from "@base-ui/react/dialog";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { IconButton } from "./IconButton";

interface EditorDialogProps {
  eyebrow: string;
  title: ReactNode;
  titleId: string;
  closeLabel: string;
  onRequestClose: () => void;
  closeDisabled?: boolean;
  wide?: boolean;
  children: ReactNode;
}

export function EditorDialog({
  eyebrow,
  title,
  titleId,
  closeLabel,
  onRequestClose,
  closeDisabled = false,
  wide = false,
  children,
}: EditorDialogProps) {
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) onRequestClose();
      }}
      disablePointerDismissal={closeDisabled}
    >
      <Dialog.Portal>
        <Dialog.Backdrop className="editor-backdrop" />
        <Dialog.Viewport className="editor-viewport">
          <Dialog.Popup className={`editor-dialog${wide ? " checkpoint-editor-dialog" : ""}`}>
            <header className="editor-header">
              <div>
                <span className="eyebrow">{eyebrow}</span>
                <Dialog.Title id={titleId}>{title}</Dialog.Title>
              </div>
              <Dialog.Close
                disabled={closeDisabled}
                render={
                  <IconButton
                    label={closeLabel}
                    icon={<X aria-hidden="true" strokeWidth={1.8} />}
                  />
                }
              />
            </header>
            {children}
          </Dialog.Popup>
        </Dialog.Viewport>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
