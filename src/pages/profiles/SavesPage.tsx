import { useProfiles } from "@/hooks";
import { SavesView } from "@/pages/profiles/SavesView";

/** The Saves tab: the active profile's save set, backups and save editor. */
export function SavesPage() {
  const active = useProfiles().data?.active;
  return active ? <SavesView key={active} profileId={active} /> : null;
}
