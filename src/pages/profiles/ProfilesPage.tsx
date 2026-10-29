import { useTranslation } from "react-i18next";
import { PageHeader, SettingsSection } from "@/components/settings";
import { useProfiles } from "@/hooks";
import { ProfileCard } from "@/pages/profiles/ProfileCard";
import { CreateProfileDialog } from "@/pages/profiles/CreateProfileDialog";
import { Message } from "@/components/common";

export function ProfilesPage() {
  const { t } = useTranslation();
  const { data, change, busy, running, error } = useProfiles();
  const disabled = busy || running;
  return (<>
    <PageHeader title={t("profiles.title")} subtitle={t("profiles.subtitle")} actions={
      <CreateProfileDialog disabled={disabled || !data} busy={busy}
        onCreate={(name) => change({ action: "create", id: "", name })} />
    } />
    {running && <Message tone="warning" className="mb-4">{t("saves.stopFirst")}</Message>}
    {error && <Message tone="error" className="mb-4">{String(error)}</Message>}
    <SettingsSection>
      {data?.profiles.map((profile) => <ProfileCard key={profile.id} profile={profile} active={profile.id === data.active} disabled={disabled} busy={busy} onChange={change} />)}
    </SettingsSection>
  </>);
}
