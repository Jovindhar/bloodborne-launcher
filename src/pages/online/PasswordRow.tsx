import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Input } from "@/components/ui";
import { SettingRow } from "@/components/settings";
import { useOnlinePasswordSaved, useSetOnlinePassword } from "@/hooks";

/** The Online ID's password: typed here, kept in the macOS Keychain, never shown again. */
export function PasswordRow({ npid }: { npid: string }) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");
  const { data: saved } = useOnlinePasswordSaved(npid);
  const setPassword = useSetOnlinePassword(npid);
  const save = () => draft && setPassword.mutate(draft, { onSuccess: () => setDraft("") });
  return (
    <SettingRow
      title={t("online.password")}
      description={!npid ? t("online.passwordNeedsId") : saved ? t("online.passwordIsSaved") : t("online.passwordHint")}
      disabled={!npid}
    >
      <Input
        type="password"
        autoComplete="off"
        className="w-52 bg-secondary text-xs"
        value={draft}
        placeholder={saved ? "••••••••" : ""}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && save()}
      />
      <Button variant="secondary" size="sm" onClick={save} disabled={!draft || setPassword.isPending}>
        {t("online.savePassword")}
      </Button>
      {saved && (
        <Button variant="ghost" size="sm" onClick={() => setPassword.mutate("")} disabled={setPassword.isPending}>
          {t("online.forgetPassword")}
        </Button>
      )}
    </SettingRow>
  );
}
