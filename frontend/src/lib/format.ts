const short: Intl.DateTimeFormatOptions = {
  month: "short",
  day: "numeric",
  timeZone: "UTC",
};

const short_date_format = new Intl.DateTimeFormat(undefined, short);
const short_time_format = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
});
const clock_format = new Intl.DateTimeFormat(undefined, {
  hour: "numeric",
  minute: "2-digit",
});
const long_date_format = new Intl.DateTimeFormat(undefined, {
  ...short,
  year: "numeric",
});
const one_decimal = new Intl.NumberFormat(undefined, {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});
const up_to_one_decimal = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 1,
});

export const short_date = (date: string) =>
  short_date_format.format(new Date(date));

export const short_time = (date: string) =>
  short_time_format.format(new Date(date));

export const clock_time = (ms: number) => clock_format.format(ms);

export const long_date = (date: string) =>
  long_date_format.format(new Date(date));

export const duration = (hours: number | null) => {
  if (hours === null) return "–";
  const minutes = Math.round(hours * 60);
  if (minutes < 60) return `${minutes}m`;
  const whole = Math.round(hours);
  return whole < 48
    ? `${whole.toLocaleString()}h`
    : `${one_decimal.format(hours / 24)}d`;
};

export const hours = (seconds: number) => {
  const text = up_to_one_decimal.format(seconds / 3600);
  return `${text} ${Math.round((seconds / 3600) * 10) === 10 ? "hour" : "hours"}`;
};

export const age = (ms: number) => {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  if (seconds < 10) return "just now";
  if (seconds < 60) return `${Math.floor(seconds / 10) * 10}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m ago`;
};

export const days_left = (date: string) => {
  const days = Math.max(0, Math.ceil((Date.parse(date) - Date.now()) / 86400000));
  return `${days} day${days === 1 ? "" : "s"}`;
};
