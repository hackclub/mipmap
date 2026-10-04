const short: Intl.DateTimeFormatOptions = {
  month: "short",
  day: "numeric",
  timeZone: "UTC",
};

export const short_date = (date: string) =>
  new Date(date).toLocaleDateString(undefined, short);

export const long_date = (date: string) =>
  new Date(date).toLocaleDateString(undefined, { ...short, year: "numeric" });

export const hours = (seconds: number) =>
  `${(seconds / 3600).toFixed(1)} hours`;
