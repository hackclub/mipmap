import all from "$lib/assets/challenges.json";

type Challenge = {
  name: string;
  description: string;
  start: string;
  end: string;
};

const zone = new Intl.DateTimeFormat("en-US", {
  timeZone: "America/New_York",
  hourCycle: "h23",
  year: "numeric",
  month: "numeric",
  day: "numeric",
  hour: "numeric",
  minute: "numeric",
  second: "numeric",
});

function offset(time: number) {
  const parts = Object.fromEntries(
    zone.formatToParts(time).map((part) => [part.type, Number(part.value)]),
  );
  const wall = Date.UTC(
    parts.year,
    parts.month - 1,
    parts.day,
    parts.hour,
    parts.minute,
    parts.second,
  );
  return wall - Math.floor(time / 1000) * 1000;
}

function eastern_time(value: string, end: boolean) {
  const [date, time = "00:00"] = value.split("T");
  const [year, month, day] = date.split("-").map(Number);
  const [hours, minutes] = time.split(":").map(Number);
  const wall = Date.UTC(
    year,
    month - 1,
    end && value === date ? day + 1 : day,
    hours,
    minutes,
  );
  const guess = wall - offset(wall);
  return wall - offset(guess);
}

export const challenges = (all as Challenge[]).map((challenge) => ({
  ...challenge,
  starts: eastern_time(challenge.start, false),
  ends: eastern_time(challenge.end, true),
}));
