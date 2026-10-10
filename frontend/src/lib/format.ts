const short: Intl.DateTimeFormatOptions = {
  month: "short",
  day: "numeric",
  timeZone: "UTC",
};

export const short_date = (date: string) =>
  new Date(date).toLocaleDateString(undefined, short);

export const short_time = (date: string) =>
  new Date(date).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });

export const long_date = (date: string) =>
  new Date(date).toLocaleDateString(undefined, { ...short, year: "numeric" });

export const duration = (hours: number | null) =>
  hours === null
    ? "–"
    : hours < 48
      ? `${Math.round(hours)}h`
      : `${(hours / 24).toFixed(1)}d`;

export const hours = (seconds: number) =>
  `${(seconds / 3600).toFixed(1)} hours`;
