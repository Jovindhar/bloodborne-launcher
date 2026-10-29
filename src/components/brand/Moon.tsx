import { useId } from "react";

/** The crescent of the app icon (launcher/macos/make_icon.swift). Each instance has its own
 *  gradient and mask ids: a hidden copy (the sidebar on narrow screens) must not own them. */
export function Moon({ className }: { className?: string }) {
  const id = useId();
  return (
    <svg viewBox="0 0 64 64" className={className} aria-hidden="true">
      <defs>
        <radialGradient id={`${id}-glow`} cx="50%" cy="50%" r="50%">
          <stop offset="55%" stopColor="#f3e2c4" stopOpacity="0.25" />
          <stop offset="100%" stopColor="#f3e2c4" stopOpacity="0" />
        </radialGradient>
        <mask id={`${id}-cut`}>
          <rect width="64" height="64" fill="white" />
          <circle cx="39" cy="26" r="17" fill="black" />
        </mask>
      </defs>
      <circle cx="32" cy="32" r="30" fill={`url(#${id}-glow)`} />
      <circle cx="32" cy="32" r="20" fill="#f5e8cf" mask={`url(#${id}-cut)`} />
    </svg>
  );
}
