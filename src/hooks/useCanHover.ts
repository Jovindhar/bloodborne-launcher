import { useSyncExternalStore } from "react";

const query = "(hover: hover) and (pointer: fine)";
const subscribe = (change: () => void) => {
  const list = window.matchMedia(query);
  list.addEventListener("change", change);
  return () => list.removeEventListener("change", change);
};

/** Whether the pointer can hover (a mouse or trackpad): touch screens get taps instead of hover cards. */
export function useCanHover() {
  return useSyncExternalStore(subscribe, () => window.matchMedia(query).matches, () => true);
}
