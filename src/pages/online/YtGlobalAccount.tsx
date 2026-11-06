import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { SettingRow, SettingsSection } from "@/components/settings";
import { AlertDialog, AlertDialogAction, AlertDialogContent, AlertDialogDescription, AlertDialogFooter,
  AlertDialogHeader, AlertDialogTitle, Button, Input } from "@/components/ui";
import { api, queryKeys } from "@/lib";

/** Yharnam Together's Global server: sign in or create an account. The password goes to that server
 *  only (over HTTPS) and is not kept; the launcher keeps the server's 30-day sign-in. */
export function YtGlobalAccount({ name }: { name: string }) {
  const { t } = useTranslation();
  const client = useQueryClient();
  const [account, setAccount] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [code, setCode] = useState("");
  const done = () => client.invalidateQueries({ queryKey: queryKeys.config });
  const run = (call: () => Promise<unknown>) => {
    setBusy(true);
    call().then(() => setPassword(""))
      .catch((error) => toast.error(t("online.ytSignInFailed"), { description: String(error) }))
      .finally(() => { setBusy(false); void done(); });
  };
  const signIn = () => run(() => api.ytSignIn(account, password));
  const signUp = () => run(() => api.ytSignUp(account, password).then((recovery) => setCode(recovery)));
  return <SettingsSection title={t("online.ytGlobalAccount")} description={t("online.ytGlobalAccountHint")}>
    {name ? <SettingRow title={t("online.ytSignedIn", { name })}>
      <Button variant="secondary" disabled={busy} onClick={() => run(() => api.ytSignOut())}>
        {t("online.ytSignOut")}
      </Button>
    </SettingRow> : <>
      <SettingRow title={t("online.ytName")} description={t("online.ytNameHint")}>
        <Input className="w-full bg-secondary font-mono text-xs sm:w-80" spellCheck={false} autoComplete="username"
          value={account} onChange={(e) => setAccount(e.target.value)} />
      </SettingRow>
      <SettingRow title={t("online.password")} description={t("online.ytPasswordHint")}>
        <Input type="password" className="w-full bg-secondary font-mono text-xs sm:w-80" autoComplete="current-password"
          value={password} onChange={(e) => setPassword(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && account && password && !busy && signIn()} />
      </SettingRow>
      <SettingRow title="">
        <div className="flex flex-wrap gap-2">
          <Button disabled={busy || !account || !password} onClick={signIn}>{t("online.ytSignIn")}</Button>
          <Button variant="secondary" disabled={busy || !account || !password} onClick={signUp}>
            {busy ? t("online.ytWorking") : t("online.ytCreate")}
          </Button>
        </div>
      </SettingRow>
    </>}
    <AlertDialog open={code !== ""} onOpenChange={(open) => { if (!open) setCode(""); }}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("online.ytRecoveryTitle")}</AlertDialogTitle>
          <AlertDialogDescription>{t("online.ytRecoveryText")}</AlertDialogDescription>
        </AlertDialogHeader>
        <p className="select-all rounded bg-secondary p-3 text-center font-mono text-sm">{code}</p>
        <AlertDialogFooter>
          <AlertDialogAction onClick={() => setCode("")}>{t("online.ytRecoveryDone")}</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </SettingsSection>;
}
