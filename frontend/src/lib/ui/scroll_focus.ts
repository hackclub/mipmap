type Name = { label?: string; labelledby?: string };

export function scroll_region(name: Name = {}) {
  return (node: HTMLElement) => {
    const added: string[] = [];
    const set = (attribute: string, value: string | null | undefined) => {
      if (!value || node.hasAttribute(attribute)) return;
      node.setAttribute(attribute, value);
      added.push(attribute);
    };
    const named = () =>
      node.hasAttribute("aria-label") || node.hasAttribute("aria-labelledby");
    const update = () => {
      const overflowing =
        node.scrollHeight > node.clientHeight + 1 ||
        node.scrollWidth > node.clientWidth + 1;
      if (overflowing) {
        node.tabIndex = 0;
        if (!named()) {
          if (name.label) set("aria-label", name.label);
          else
            set(
              "aria-labelledby",
              name.labelledby ??
                node.parentElement
                  ?.closest("[aria-labelledby]")
                  ?.getAttribute("aria-labelledby"),
            );
        }
        if (named()) set("role", "region");
      } else {
        node.removeAttribute("tabindex");
        added.splice(0).forEach((attribute) => node.removeAttribute(attribute));
      }
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
  };
}

export const scroll_focus = scroll_region();
