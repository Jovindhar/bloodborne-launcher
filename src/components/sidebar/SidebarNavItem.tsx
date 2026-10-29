import type { ReactNode } from "react";
import { cn } from "@/lib";

type Props = { label: string; icon: ReactNode; active: boolean; onClick: () => void; badge?: ReactNode;
  /** The narrow-screen strip's form: a pill that does not shrink. */
  pill?: boolean };

/** A tab in the menu: the sidebar's row, or a pill in the narrow-screen strip. */
export function SidebarNavItem({ label, icon, active, onClick, badge, pill = false }: Props) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex items-center text-sm transition [&_svg]:size-4",
        pill ? "shrink-0 gap-1.5 rounded-full px-3.5 py-2" : "gap-3 rounded-lg px-3 py-2",
        active
          ? "bg-sidebar-accent text-sidebar-accent-foreground [&_svg]:text-gold-400"
          : cn("text-muted-foreground", pill ? "active:bg-white/5" : "hover:bg-white/5 hover:text-foreground"),
      )}
    >
      {icon}
      {label}
      {badge && <span className={pill ? undefined : "ml-auto"}>{badge}</span>}
    </button>
  );
}

