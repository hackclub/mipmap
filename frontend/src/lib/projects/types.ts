export type Tier = "quebec" | "romeo" | "sierra";

export interface ProjectFields {
  name: string;
  description: string;
  repo: string;
  demo: string;
  tier: Tier | null;
  hackatime: string[];
}

export interface Project extends ProjectFields {
  id: number;
  screenshot: string | null;
}

export interface HackatimeProject {
  name: string;
  seconds: number;
}
