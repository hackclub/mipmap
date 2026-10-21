export type Day = {
  date: string;
  signed_up: number;
  active: number;
  logged_in: number;
};

export type Hour = {
  hour: string;
  signed_up: number | null;
  logged_in: number | null;
};

export type Stats = {
  days: Day[];
  hours: Hour[];
  totals: {
    hours_building: number;
    projects_created: number;
    projects_shipped: number;
    hours_approved: number;
    mips_awarded: number;
    snapshots_shared: number;
    successful_referrals: number;
    orders_placed: number;
    logged_in_last_day: number | null;
    logged_in_last_week: number;
    logged_in_ever: number;
    awaiting_review: number;
    average_wait_hours: number | null;
    average_approval_hours: number | null;
  };
  funnel: Record<"day" | "week" | "all", Funnel>;
  onboarding: Record<"day" | "week" | "all", Onboarding>;
};

export type Onboarding = {
  users: number | null;
  started: number | null;
  made_project: number | null;
  reached_tech_tree: number | null;
  unlocked_sticker: number | null;
  stickers: number | null;
  finished: number | null;
  in_progress: number | null;
  median_minutes: number | null;
};

export type Funnel = {
  visited: number | null;
  signed_in: number | null;
  linked_hackatime: number | null;
  created_project: number | null;
  logged_time: number | null;
  shipped: number | null;
  redeemed: number | null;
};
