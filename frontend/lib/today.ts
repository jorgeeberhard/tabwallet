/**
 * Today's calendar day (`YYYY-MM-DD`) in America/Sao_Paulo, the same "today"
 * the API uses to decide what is a Projection. The browser's own time zone
 * is deliberately ignored.
 */
export function todayInSaoPaulo(now: Date = new Date()): string {
  // The "en-CA" locale formats a date as YYYY-MM-DD.
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Sao_Paulo",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now)
}
