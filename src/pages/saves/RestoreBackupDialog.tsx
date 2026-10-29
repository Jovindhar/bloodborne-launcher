import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui";
import { Message } from "@/components/common";

export function RestoreBackupDialog({ label, disabled, onRestore }: {
  label: string; disabled: boolean; onRestore: () => Promise<void>;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string>();
  return <Dialog open={open} onOpenChange={(next, details) => {
    if (pending) { details.cancel(); return; }
    setOpen(next);
    setError(undefined);
  }}>
    <DialogTrigger render={<Button variant="secondary" size="sm" disabled={disabled} />}>{t("saves.restore")}</DialogTrigger>
    <DialogContent showCloseButton={!pending}>
      <DialogHeader>
        <DialogTitle>{t("saves.restore")}</DialogTitle>
        <DialogDescription>{t("saves.restoreHint")}</DialogDescription>
      </DialogHeader>
      <p className="text-sm text-muted-foreground">{label}</p>
      {error && <Message tone="error">{error}</Message>}
      <DialogFooter>
        <Button variant="outline" disabled={pending} onClick={() => setOpen(false)}>{t("saves.cancel")}</Button>
        <Button variant="destructive" disabled={disabled || pending} onClick={async () => {
          setPending(true);
          setError(undefined);
          try { await onRestore(); setOpen(false); }
          catch (failure) { setError(String(failure)); }
          finally { setPending(false); }
        }}>{t("saves.restore")}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>;
}
