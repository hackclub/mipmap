export function scroll_focus(node: HTMLElement) {
  const update = () => {
    const overflowing =
      node.scrollHeight > node.clientHeight + 1 ||
      node.scrollWidth > node.clientWidth + 1;
    if (overflowing) node.tabIndex = 0;
    else node.removeAttribute("tabindex");
  };
  const resize = new ResizeObserver(update);
  const watch = () => {
    resize.disconnect();
    resize.observe(node);
    for (const child of node.children) resize.observe(child);
  };
  const children = new MutationObserver(watch);
  children.observe(node, { childList: true });
  watch();
  return () => {
    resize.disconnect();
    children.disconnect();
  };
}
