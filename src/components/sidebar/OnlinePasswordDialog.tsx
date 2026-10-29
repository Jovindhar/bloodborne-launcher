import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input } from "@/components/ui";

/** Asks for the online password at launch when it is not saved; it goes to this run only. */
export function OnlinePasswordDialog({ open, account, onPlay, onCancel }: {
  open: boolean;
  account: string;
  onPlay: (password: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const [password, setPassword] = useState("");
  const close = () => { setPassword(""); onCancel(); };
  return (
    <Dialog open={open} onOpenChange={(next) => { if (!next) close(); }}>
      <DialogContent className="sm:max-w-sm">
        <form onSubmit={(event) => {
          event.preventDefault();
          if (!password) return;
          const typed = password;
          setPassword("");
          onPlay(typed);
        }}>
          <DialogHeader>
            <DialogTitle>{t("online.passwordPromptTitle")}</DialogTitle>
            <DialogDescription>{t("online.passwordPromptHint", { account })}</DialogDescription>
          </DialogHeader>
          <Input type="password" autoFocus autoComplete="off" spellCheck={false} className="my-4"
            aria-label={t("online.password")} value={password} onChange={(event) => setPassword(event.target.value)} />
          <DialogFooter>
            <Button type="button" variant="secondary" onClick={close}>{t("saves.cancel")}</Button>
            <Button type="submit" disabled={!password}>{t("play.play")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
