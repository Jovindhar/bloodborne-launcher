import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription,
  AlertDialogFooter, AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger, Button } from "@/components/ui";
import type { SaveProfile } from "@/lib";
import { Message } from "@/components/common";

export function DeleteProfileDialog({ profile, active, disabled, busy, onDelete }: {
  profile: SaveProfile;
  active: boolean;
  disabled: boolean;
  busy: boolean;
  onDelete: () => Promise<void>;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string>();
  const cancel = useRef<HTMLButtonElement>(null);

  return (
    <AlertDialog open={open} onOpenChange={(next, details) => {
      if (!next && busy) { details.cancel(); return; }
      setOpen(next);
      if (!next) setError(undefined);
    }}>
      <span title={active ? t("profiles.deleteActiveHint") : undefined}>
        <AlertDialogTrigger render={<Button size="sm" variant="destructive" disabled={disabled || active} />}
          aria-label={t("profiles.deleteTitle", { name: profile.name })}>
          {t("profiles.delete")}
        </AlertDialogTrigger>
      </span>
      <AlertDialogContent initialFocus={cancel}>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("profiles.deleteTitle", { name: profile.name })}</AlertDialogTitle>
          <AlertDialogDescription>{t("profiles.deleteHint", { name: profile.name })}</AlertDialogDescription>
        </AlertDialogHeader>
        {error && <Message tone="error">{error}</Message>}
        <AlertDialogFooter>
          <AlertDialogCancel ref={cancel} disabled={busy}>{t("saves.cancel")}</AlertDialogCancel>
          <AlertDialogAction variant="destructive" disabled={disabled || active} onClick={async () => {
            if (disabled || active) return;
            setError(undefined);
            try { await onDelete(); setOpen(false); }
            catch (failure) { setError(String(failure)); }
          }}>{t("profiles.delete")}</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
