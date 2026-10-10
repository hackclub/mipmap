export type Day = {
  date: string;
  signed_up: number;
  active: number;
};

export type Hour = {
  hour: string;
  signed_up: number;
  logged_in: number;
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
    logged_in_last_day: number;
    awaiting_review: number;
    average_wait_hours: number | null;
    average_approval_hours: number | null;
  };
  funnel: {
    visited: number;
    signed_in: number;
    linked_hackatime: number;
    created_project: number;
    logged_time: number;
    shipped: number;
    redeemed: number;
  };
};
