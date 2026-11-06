import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { PageHeader, SelectRow, SettingsSection, SwitchRow, TextRow } from "@/components/settings";
import { useConfig, useProfiles, useSetting, useSettingsData } from "@/hooks";
import { api, queryKeys } from "@/lib";
import { SavePasswordDialog } from "@/pages/online/SavePasswordDialog";
import { YtGlobalAccount } from "@/pages/online/YtGlobalAccount";
import { Message } from "@/components/common";

/** Accounts belong to the selected save profile; the backend blocks edited saves at launch. */
export function OnlinePage() {
  const { t } = useTranslation();
  const { settings } = useSettingsData();
  const set = useSetting();
  const { data: profiles } = useProfiles();
  const active = profiles?.profiles.find((profile) => profile.id === profiles.active);
  const edited = active && !active.trophies_eligible;
  const thd = settings.online_pool === "huntersdream";
  const yt = settings.online_pool === "yharnamtogether";
  const { data: config } = useConfig();
  const client = useQueryClient();
  const [confirmSave, setConfirmSave] = useState(false);
  const savePassword = (password: string) => api.saveOnlinePassword(password)
    .catch((error) => toast.error(t("common.saveFailed"), { description: String(error) }))
    .finally(() => client.invalidateQueries({ queryKey: queryKeys.config }));
  const switchSaving = (save: boolean) => {
    if (save) { setConfirmSave(true); return; }
    set("online_save_password", false); // the backend deletes the saved password
    void client.invalidateQueries({ queryKey: queryKeys.config });
  };
  return <>
    <PageHeader title={t("online.title")} subtitle={t("online.subtitle")} />
    {edited && <Message tone="warning" className="mb-4">{t("online.editedSave")}</Message>}
    <SettingsSection title={t("online.play") }>
      <SwitchRow title={t("online.enable")} description={t("online.enableHint")}
        checked={settings.online} onChange={(value) => set("online", value)} />
      <SelectRow title={t("online.pool")} description={t("online.poolHint")}
        value={settings.online_pool}
        options={[{ value: "seamless", label: t("online.seamless"), literal: true },
                  { value: "huntersdream", label: t("online.huntersDream"), literal: true },
                  { value: "yharnamtogether", label: t("online.yharnamTogether"), literal: true }]}
        onChange={(value) => set("online_pool", value)} />
    </SettingsSection>
    {/* BloodOnline: Yharnam Together (the runtime's gpu/ytcoop module): its own seamless co-op. */}
    {yt ? <>
      <SettingsSection title={t("online.yharnamTogether")} description={t("online.ytHint")}>
        <SelectRow title={t("online.ytServer")} value={settings.yt_server_type}
          options={[{ value: "global", label: "online.ytGlobal", hint: "online.ytGlobalHint" },
                    { value: "local", label: "online.ytLocal", hint: "online.ytLocalHint" }]}
          onChange={(value) => set("yt_server_type", value)} />
        {settings.yt_server_type === "local" &&
          <TextRow title={t("online.ytHost")} description={t("online.ytHostHint")} value={settings.yt_host}
            placeholder="192.168.1.20" onChange={(value) => set("yt_host", value.trim())} />}
      </SettingsSection>
      {settings.yt_server_type !== "local" && <YtGlobalAccount name={config?.yt_global_name ?? ""} />}
    </> : <>
    {/* BloodOnline: BBSC seamless co-op (CLS_AMG, from bbport 0.6), with Seamless Hunt only. */}
    <SettingsSection title={t("online.seamlessCoop")} description={t("online.bbscHint")}>
      <SwitchRow title={t("online.bbsc")} description={thd ? t("online.bbscSeamlessOnly") : undefined}
        checked={settings.online_coop} disabled={thd} onChange={(value) => set("online_coop", value)} />
      <SelectRow title={t("online.localServer")} value={settings.online_local_server}
        options={[{ value: "auto", label: "common.auto", hint: "online.localServerAuto" },
                  { value: "on", label: "common.on", hint: "online.localServerOn" },
                  { value: "off", label: "common.off", hint: "online.localServerOff" }]}
        disabled={thd || !settings.online_coop} onChange={(value) => set("online_local_server", value)} />
    </SettingsSection>
    {thd ? <>
      <SettingsSection title={t("online.thdAccount")} description={t("online.thdAccountHint")}>
        <TextRow title={t("online.onlineId")} value={settings.thd_online_id}
          onChange={(value) => set("thd_online_id", value)} />
        <TextRow title={t("online.token")} type="password" value={settings.thd_token}
          onChange={(value) => set("thd_token", value)} />
        <TextRow title={t("online.thdServer")} value={settings.thd_server}
          onChange={(value) => set("thd_server", value)} />
      </SettingsSection>
    </> : <>
      <SettingsSection title={t("online.seamlessAccount")} description={t("online.seamlessAccountHint")}>
        <TextRow title={t("online.onlineId")} value={settings.online_npid}
          onChange={(value) => set("online_npid", value)} />
        <SwitchRow title={t("online.savePassword")}
          description={settings.online_save_password ? t("online.savePasswordOn") : t("online.savePasswordOff")}
          checked={settings.online_save_password} onChange={switchSaving} />
        {settings.online_save_password &&
          <TextRow title={t("online.password")} type="password" value=""
            placeholder={config?.online_password_saved ? t("online.passwordSaved") : undefined}
            onChange={(value) => void savePassword(value)} />}
      </SettingsSection>
      <SettingsSection title={t("online.coop")} description={t("online.coopHint")}>
        <SwitchRow title={t("online.upnp")} description={t("online.upnpHint")}
          checked={settings.online_upnp} onChange={(value) => set("online_upnp", value)} />
        <TextRow title={t("online.server")} value={settings.online_server}
          onChange={(value) => set("online_server", value)} />
        <TextRow title={t("online.webapi")} value={settings.online_webapi}
          onChange={(value) => set("online_webapi", value)} />
        <TextRow title={t("online.gameServer")} value={settings.online_community}
          onChange={(value) => set("online_community", value)} />
      </SettingsSection>
    </>}
    </>}
    <SavePasswordDialog open={confirmSave}
      onAccept={() => { setConfirmSave(false); set("online_save_password", true); }}
      onDecline={() => setConfirmSave(false)} />
  </>;
}
