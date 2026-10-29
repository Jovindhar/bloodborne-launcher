import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib";
import { cn } from "cn";

/** An item's icon, decoded from the player's game files (a blank square until it loads). */
export function ItemIcon({ icon, className }: { icon: number | null; className?: string }) {
  const { data } = useQuery({
    queryKey: ["item-icon", icon],
    queryFn: () => api.itemIcon(icon ?? 0),
    enabled: icon !== null,
    staleTime: Infinity,
    retry: false,
  });
  return <div className={cn("size-10 shrink-0 overflow-hidden rounded-md bg-muted", className)}>
    {data && <img src={data} alt="" className="size-full object-contain" draggable={false} />}
  </div>;
}
