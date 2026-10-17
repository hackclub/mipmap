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
  status: "draft" | "shipped" | "approved" | "rejected";
  awaiting_id: boolean;
  locked: boolean;
  shipped_at: string | null;
  mips_awarded: number;
}

export interface CheckEvent {
  check: string;
  status: "running" | "passed" | "unverified" | "failed";
  label?: string;
  message?: string;
}

export interface HackatimeProject {
  name: string;
  seconds: number;
}
