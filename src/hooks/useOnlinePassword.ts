import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api, queryKeys } from "@/lib";
import { i18n } from "@/i18n";

/** Whether the Online ID has a password in the Keychain (the password never comes back). */
export function useOnlinePasswordSaved(npid: string) {
  return useQuery({ queryKey: queryKeys.onlinePassword(npid), queryFn: () => api.onlinePasswordSaved(npid), enabled: npid !== "" });
}

/** Saves the Online ID's password in the Keychain; an empty password forgets it. */
export function useSetOnlinePassword(npid: string) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (password: string) => api.setOnlinePassword(npid, password),
    onSuccess: (_data, password) => {
      toast.success(i18n.t(password ? "online.passwordSaved" : "online.passwordForgotten"));
      client.invalidateQueries({ queryKey: queryKeys.onlinePassword(npid) });
    },
    onError: (error) => toast.error(i18n.t("common.saveFailed"), { description: String(error) }),
  });
}
