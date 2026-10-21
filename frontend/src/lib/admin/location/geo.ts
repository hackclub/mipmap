import type { Feature, Geometry } from "geojson";
import numeric from "$lib/assets/iso_numeric.json";
import points from "$lib/assets/small_countries.json";

export type Country = Feature<Geometry, { name: string }> & { code?: string };

const names = new Intl.DisplayNames(["en"], { type: "region" });

export function country_name(code: string) {
  try {
    return names.of(code) ?? code;
  } catch {
    return code;
  }
}

const hex = (color: string) =>
  [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16));

function mix(a: string, b: string, t: number) {
  const [x, y] = [hex(a), hex(b)];
  return `rgb(${x.map((v, i) => Math.round(v + (y[i] - v) * t)).join(" ")})`;
}

const stops = ["#74472a", "#d6ab70", "#ead7c2"];

export function ramp(t: number) {
  const at = Math.min(1, Math.max(0, t)) * (stops.length - 1);
  const i = Math.min(stops.length - 2, Math.floor(at));
  return mix(stops[i], stops[i + 1], at - i);
}

export const dim = (t: number) => ramp(0.25 + t * 0.35);

export const scale = (users: number, max: number) =>
  max <= 0 ? 0 : Math.log1p(users) / Math.log1p(max);

export const small = points as unknown as Record<string, [number, number]>;

let world: Promise<Country[]> | undefined;

export function load_world() {
  world ??= Promise.all([
    import("topojson-client"),
    import("world-atlas/countries-110m.json"),
  ]).then(([topojson, atlas]) => {
    const topology = atlas.default as unknown as Parameters<
      typeof topojson.feature
    >[0];
    const collection = topojson.feature(
      topology,
      topology.objects.countries,
    ) as unknown as { features: Country[] };
    for (const feature of collection.features)
      feature.code = (numeric as Record<string, string>)[String(feature.id)];
    return collection.features;
  });
  return world;
}

const us_zones: Record<string, string> = {};
for (const [zone, states] of Object.entries({
  "America/New_York":
    "CT Connecticut DE Delaware DC District_of_Columbia FL Florida GA Georgia IN Indiana KY Kentucky ME Maine MD Maryland MA Massachusetts MI Michigan NH New_Hampshire NJ New_Jersey NY New_York NC North_Carolina OH Ohio PA Pennsylvania RI Rhode_Island SC South_Carolina VT Vermont VA Virginia WV West_Virginia",
  "America/Chicago":
    "AL Alabama AR Arkansas IL Illinois IA Iowa KS Kansas LA Louisiana MN Minnesota MS Mississippi MO Missouri NE Nebraska ND North_Dakota OK Oklahoma SD South_Dakota TN Tennessee TX Texas WI Wisconsin",
  "America/Denver":
    "CO Colorado ID Idaho MT Montana NM New_Mexico UT Utah WY Wyoming",
  "America/Phoenix": "AZ Arizona",
  "America/Los_Angeles": "CA California NV Nevada OR Oregon WA Washington",
  "America/Anchorage": "AK Alaska",
  "Pacific/Honolulu": "HI Hawaii",
}))
  for (const state of states.split(" "))
    us_zones[state.replace(/_/g, " ").toLowerCase()] = zone;

export const us_zone = (region: string) =>
  us_zones[region.trim().toLowerCase()] ?? "America/Chicago";

export function zone_hour(zone: string, utc: Date) {
  const hour = new Intl.DateTimeFormat("en-US", {
    timeZone: zone,
    hour: "numeric",
    hourCycle: "h23",
  }).format(utc);
  return Number(hour);
}
