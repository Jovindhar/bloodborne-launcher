import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ScrollText, X } from "lucide-react";
import { Alert, AlertDescription, AlertTitle, Button } from "@/components/ui";
import { UpdateDialog } from "@/components/updates/UpdateDialog";
import { useRelease } from "@/hooks";

export function UpdateNotice() {
  const { t } = useTranslation();
  const { data: release } = useRelease();
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [closedDialog, setClosedDialog] = useState<string | null>(null);
  if (!release?.newer || dismissed === release.tag) return null;
  return (
    <>
      <Alert className="mb-5 bg-primary/15">
        <AlertTitle>{t("updates.available", { version: release.version })}</AlertTitle>
        <AlertDescription className="flex flex-wrap items-center justify-between gap-3">
          <span>{t("updates.downloadHint")}</span>
          <div className="flex items-center gap-2">
            <Button size="sm" onClick={() => setClosedDialog(null)}><ScrollText />{t("updates.changelog")}</Button>
            <Button size="icon-sm" variant="ghost" aria-label={t("updates.dismiss")} onClick={() => setDismissed(release.tag)}><X /></Button>
          </div>
        </AlertDescription>
      </Alert>
      <UpdateDialog release={release} open={closedDialog !== release.tag}
        onOpenChange={(open) => setClosedDialog(open ? null : release.tag)} />
    </>
  );
}
