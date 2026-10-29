import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api } from "@/lib";
import { useConfig } from "@/hooks";

/** The items the player's game files name, read once per game folder and language: the editor's
 * names, icons and numbers all come from the same files, so it needs this list to open. */
export function useItemCatalog() {
  const { i18n } = useTranslation();
  const game = useConfig().data?.settings.game_dir ?? "";
  return useQuery({
    queryKey: ["item-catalog", game, i18n.language],
    queryFn: () => api.itemCatalog(i18n.language),
    enabled: game !== "",
    staleTime: Infinity,
    retry: false,
  });
}
