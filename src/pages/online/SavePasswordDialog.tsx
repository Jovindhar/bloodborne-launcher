import { useTranslation } from "react-i18next";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription,
  AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui";

/** The warning before the password is saved: on this computer, unencrypted. Declined: asked at each launch. */
export function SavePasswordDialog({ open, onAccept, onDecline }: { open: boolean; onAccept: () => void; onDecline: () => void }) {
  const { t } = useTranslation();
  return (
    <AlertDialog open={open} onOpenChange={(next) => { if (!next) onDecline(); }}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("online.savePasswordTitle")}</AlertDialogTitle>
          <AlertDialogDescription>{t("online.savePasswordWarning")}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={onDecline}>{t("online.dontSave")}</AlertDialogCancel>
          <AlertDialogAction onClick={onAccept}>{t("online.savePasswordAccept")}</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
