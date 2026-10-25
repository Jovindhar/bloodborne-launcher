import { useTranslation } from "react-i18next";
import { ExternalLink } from "lucide-react";
import { Button } from "@/components/ui";
import { PageHeader, SelectRow, SettingRow, SettingsSection, SwitchRow, TextRow } from "@/components/settings";
import { useSetting, useSettingsData } from "@/hooks";
import { api } from "@/lib";
import type { Option, Settings } from "@/lib";
import { PasswordRow } from "@/pages/online/PasswordRow";

const ACCOUNT_SITE = "https://seamlesshunt.com";

const LOCAL_SERVER: Option<Settings["online_local_server"]>[] = [
  { value: "auto", label: "common.auto", hint: "online.localServerAuto" },
  { value: "on", label: "common.on", hint: "online.localServerOn" },
  { value: "off", label: "common.off", hint: "online.localServerOff" },
];

/** Online play through Seamless Hunt (bbport 0.6's online module): account and co-op. */
export function OnlinePage() {
  const { t } = useTranslation();
  const { settings } = useSettingsData();
  const setSetting = useSetting();
  const online = settings.online ?? false;
  const npid = (settings.online_npid ?? "").trim();
  return (
    <>
      <PageHeader title={t("online.title")} subtitle={t("online.subtitle")} />
      <SettingsSection description={t("common.restartNote")}>
        <SwitchRow title={t("online.enabled")} description={t("online.enabledHint")} checked={online}
          onChange={(v) => setSetting("online", v)} />
      </SettingsSection>
      <SettingsSection title={t("online.account")} description={t("online.accountHint")}>
        <SettingRow title={t("online.createAccount")} description={t("online.createAccountHint")}>
          <Button variant="secondary" size="sm" onClick={() => api.openPath(ACCOUNT_SITE)}>
            seamlesshunt.com <ExternalLink aria-hidden="true" />
          </Button>
        </SettingRow>
        <TextRow key={npid} title={t("online.npid")} description={t("online.npidHint")} value={npid}
          onChange={(v) => setSetting("online_npid", v.trim())} />
        <PasswordRow key={`password-${npid}`} npid={npid} />
      </SettingsSection>
      <SettingsSection title={t("online.coop")} description={t("online.coopHint")}>
        <SwitchRow title={t("online.bbsc")} description={t("online.bbscHint")} checked={settings.online_coop ?? true}
          onChange={(v) => setSetting("online_coop", v)} />
        <SwitchRow title={t("online.upnp")} description={t("online.upnpHint")} checked={settings.online_upnp ?? true}
          onChange={(v) => setSetting("online_upnp", v)} />
        <SelectRow title={t("online.localServer")} value={settings.online_local_server ?? "auto"} options={LOCAL_SERVER}
          onChange={(v) => setSetting("online_local_server", v)} disabled={!(settings.online_coop ?? true)} />
      </SettingsSection>
      <SettingsSection title={t("online.server")}>
        <TextRow title={t("online.serverAddress")} description={t("online.serverAddressHint")}
          value={settings.online_server ?? ""} placeholder="play.seamlesshunt.com:31313"
          onChange={(v) => setSetting("online_server", v.trim())} />
      </SettingsSection>
    </>
  );
}
