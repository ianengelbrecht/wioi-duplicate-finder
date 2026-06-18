/**
 * Copies the selected text from an HTML input/textarea element, or the fallback value if no selection.
 *
 * @param {HTMLInputElement|HTMLTextAreaElement|null} element - The target input or textarea element.
 * @param {string} fallbackValue - The value to copy if there is no active selection in the element.
 * @returns {Promise<boolean>} Resolves to true if copy succeeded, false otherwise.
 */
export async function copySelectedOrValue(element, fallbackValue) {
  if (!element) return false;
  const start = element.selectionStart;
  const end = element.selectionEnd;
  let textToCopy = "";
  if (start !== null && end !== null && start !== end) {
    textToCopy = element.value.substring(start, end);
  } else {
    textToCopy = fallbackValue || "";
  }
  if (!textToCopy) return false;
  try {
    await navigator.clipboard.writeText(textToCopy);
    return true;
  } catch (err) {
    console.error("Failed to copy text:", err);
    return false;
  }
}

/**
 * Pastes text from clipboard into the input/textarea element at the current cursor position.
 *
 * @param {HTMLInputElement|HTMLTextAreaElement|null} element - The target input or textarea element.
 * @param {string} currentValue - The current value of the field.
 * @returns {Promise<{ newValue: string, newCursorPos: number }|null>} Resolves with the new value and new cursor position, or null if copy/paste failed.
 */
export async function pasteAtCursor(element, currentValue) {
  if (!element) return null;
  try {
    const clipboardText = await navigator.clipboard.readText();
    if (!clipboardText) return null;
    const start = element.selectionStart || 0;
    const end = element.selectionEnd || 0;
    const val = currentValue || "";
    const newValue = val.substring(0, start) + clipboardText + val.substring(end);
    const newCursorPos = start + clipboardText.length;
    return { newValue, newCursorPos };
  } catch (err) {
    console.error("Failed to paste from clipboard:", err);
    return null;
  }
}
