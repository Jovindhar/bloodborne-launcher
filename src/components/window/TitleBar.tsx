/** The strip under the hidden title bar (traffic lights): drags the window. */
export function TitleBar() {
  return <div className="drag sticky top-0 z-10 hidden h-9 shrink-0 md:block" data-tauri-drag-region />;
}
