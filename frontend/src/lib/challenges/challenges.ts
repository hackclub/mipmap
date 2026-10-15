import all from "$lib/assets/challenges.json";

type Challenge = {
  name: string;
  description: string;
  start: string;
  end: string;
};

function local_time(value: string, end: boolean) {
  const [date, time] = value.split("T");
  const [year, month, day] = date.split("-").map(Number);
  if (!time) return new Date(year, month - 1, end ? day + 1 : day).getTime();
  const [hours, minutes] = time.split(":").map(Number);
  return new Date(year, month - 1, day, hours, minutes).getTime();
}

export const challenges = (all as Challenge[]).map((challenge) => ({
  ...challenge,
  starts: local_time(challenge.start, false),
  ends: local_time(challenge.end, true),
}));
