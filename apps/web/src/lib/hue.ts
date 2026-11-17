/** The colour a category gets, by its place in the list: eight hues, then round again (see theme.css). */
export function hueFor(index: number): string {
  return String(((index % 8) + 8) % 8);
}
