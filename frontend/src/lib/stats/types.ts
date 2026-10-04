export type Day = {
  date: string;
  signed_up: number;
  active: number;
};

export type Stats = {
  days: Day[];
  totals: {
    hours_building: number;
    projects_shipped: number;
    hours_approved: number;
    mips_awarded: number;
    snapshots_shared: number;
    successful_referrals: number;
    orders_placed: number;
  };
  funnel: {
    visited: number;
    signed_in: number;
    linked_hackatime: number;
    logged_time: number;
    shipped: number;
    redeemed: number;
  };
};
