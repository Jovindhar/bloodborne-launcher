import { Plus } from "lucide-react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Input, Label, Dialog, DialogTrigger, DialogContent, DialogTitle, DialogDescription, DialogClose, DialogHeader, DialogFooter } from "@/components/ui";
import { Message } from "@/components/common";

export function CreateProfileDialog({ disabled, busy, onCreate }: {
  disabled: boolean;
  busy: boolean;
  onCreate: (name: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState<string>();
  const input = useRef<HTMLInputElement>(null);

  return (
    <Dialog open={open} onOpenChange={(next, details) => {
      if (!next && busy) { details.cancel(); return; }
      setOpen(next);
      if (!next) { setName(""); setError(undefined); }
    }}>
      <DialogTrigger render={<Button variant="secondary" size="sm" disabled={disabled} />}><Plus aria-hidden="true" />{t("saves.newProfile")}</DialogTrigger>
      <DialogContent initialFocus={input} showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{t("saves.newProfile")}</DialogTitle>
          <DialogDescription>{t("saves.fresh")}</DialogDescription>
        </DialogHeader>
        <form onSubmit={async (event) => {
          event.preventDefault();
          if (disabled || !name.trim()) return;
          setError(undefined);
          try {
            await onCreate(name.trim());
            setOpen(false);
            setName("");
          } catch (failure) { setError(String(failure)); }
        }}>
          <Label htmlFor="profile-name">{t("saves.name")}</Label>
          <Input ref={input} id="profile-name" className="mb-4 mt-2" maxLength={80} required value={name} disabled={disabled} onChange={(event) => setName(event.target.value)} />
          {error && <Message tone="error" className="mb-4">{error}</Message>}
          <DialogFooter>
            <DialogClose render={<Button variant="outline" disabled={busy} />}>{t("saves.cancel")}</DialogClose>
            <Button type="submit" disabled={disabled || !name.trim()}>{t("saves.create")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
