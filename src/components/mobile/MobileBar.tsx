import { ActiveProfile } from "@/components/profiles";
import { PlayButton } from "@/components/sidebar";

/** Narrow screens: the profile and Play stay at the bottom, above the system's home indicator. */
export function MobileBar() {
  return (
    <div className="fixed inset-x-0 bottom-0 z-20 flex items-center gap-3 border-t border-sidebar-border bg-sidebar/95 px-4 pb-[calc(0.75rem+env(safe-area-inset-bottom))] pt-3 backdrop-blur md:hidden">
      <div className="min-w-0 flex-1"><ActiveProfile compact /></div>
      <PlayButton className="w-auto shrink-0 px-6" />
    </div>
  );
}
