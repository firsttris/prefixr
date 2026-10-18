// The dialogs and panels open on top of each other, so Escape and Tab only
// act on the topmost one: Escape in a dialog opened from the details panel
// closes that dialog, not the panel under it as well.

const stack: symbol[] = [];

/** Registers an opened layer; call the returned function when it closes. */
export function openLayer(): { id: symbol; close: () => void } {
  const id = Symbol("layer");
  stack.push(id);
  return {
    id,
    close: () => {
      const index = stack.indexOf(id);
      if (index >= 0) stack.splice(index, 1);
    },
  };
}

export function isTopLayer(id: symbol): boolean {
  return stack[stack.length - 1] === id;
}

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), summary, [tabindex]:not([tabindex="-1"])';

function focusables(container: HTMLElement): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (el) => el.getClientRects().length > 0,
  );
}

/** Keeps Tab and Shift+Tab inside `container`, cycling at its ends. */
export function trapTab(event: KeyboardEvent, container: HTMLElement): void {
  if (event.key !== "Tab") return;
  const items = focusables(container);
  const first = items[0];
  const last = items[items.length - 1];
  const active = document.activeElement;
  if (!first) {
    event.preventDefault();
    container.focus();
  } else if (!container.contains(active)) {
    event.preventDefault();
    (event.shiftKey ? last : first).focus();
  } else if (event.shiftKey && (active === first || active === container)) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}
