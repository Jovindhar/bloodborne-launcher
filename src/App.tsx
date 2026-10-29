import { ProfileRecovery, ProfileSetup } from "@/components/profiles";
import { AppWindow, LoadingScreen } from "@/components/window";
import { useConfig, useProfiles, useGameConnection, useLanguageSync, useRecompConnection, useTrayLabels } from "@/hooks";

export function App() {
  const config = useConfig();
  const { data } = config;
  const profiles = useProfiles();
  useLanguageSync();
  useTrayLabels();
  useGameConnection();
  useRecompConnection();
  if (config.error || profiles.error) return <ProfileRecovery error={config.error ?? profiles.error} />;
  if (!data || profiles.isPending) return <LoadingScreen />;
  return profiles.data?.setup_required ? <ProfileSetup /> : <AppWindow />;
}
