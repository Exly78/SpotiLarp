const TOAST_MS = 4500;
const MAX_LENGTH = 220;

/** @type {{ id: number, message: string }[]} */
export const toasts = $state([]);

let nextId = 1;

/** @param {string} message */
export function notify(message) {
  const text = message.length > MAX_LENGTH ? `${message.slice(0, MAX_LENGTH)}...` : message;
  if (toasts.some((t) => t.message === text)) return;
  const id = nextId++;
  toasts.push({ id, message: text });
  setTimeout(() => dismiss(id), TOAST_MS);
}

/** @param {number} id */
export function dismiss(id) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}
