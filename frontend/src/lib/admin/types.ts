export type Ship = {
  id: number;
  project_id: number;
  project: string;
  repo_url: string | null;
  demo_url: string | null;
  code_url: string;
  screenshot_url: string | null;
  user_id: number;
  user_name: string;
  email: string | null;
  tier: string | null;
  review_status: string | null;
  rejected: boolean;
  hours: number | null;
  tracked_hours: number | null;
  mips_per_hour: number;
  multiplier: number | null;
  mips: number;
  bonus_percent: number;
  bonus_mips: number;
  created_at: string;
  approved_at: string | null;
  awaiting_id: boolean;
  airtable_url: string | null;
};

export type Snapshot = {
  id: number;
  image: string;
  caption: string;
  author: string;
  user_id: number;
  user_name: string;
  created_at: string;
  shadowbanned: boolean;
};

export type User = {
  id: number;
  hca_id: string;
  name: string;
  email: string | null;
  slack_id: string | null;
  mips: number;
  verification_status: string | null;
  banned: boolean;
  projects: number;
  signup_at: string;
  last_activity_at: string | null;
  admin: boolean;
};

export type Order = {
  id: number;
  user_id: number;
  user_name: string;
  email: string | null;
  item_name: string;
  ordered_at: string;
  fulfilled: boolean;
  fulfilled_at: string | null;
  tracking_number: string | null;
  address_first_name: string | null;
  address_last_name: string | null;
  address_line_1: string | null;
  address_line_2: string | null;
  address_city: string | null;
  address_state: string | null;
  address_postal_code: string | null;
  address_country: string | null;
  address_phone_number: string | null;
};

export type Feedback = {
  id: number;
  user_id: number;
  user_name: string;
  score: number;
  source: string | null;
  positives: string | null;
  improvements: string | null;
  created_at: string;
};

export type Action = {
  id: number;
  admin_name: string;
  action: string;
  summary: string;
  reason: string | null;
  created_at: string;
};

export type Wave = {
  id: number;
  admin_name: string;
  created_at: string;
  stickers: number;
  unfulfilled: number;
};

export type Stickers = {
  pending: number;
  waves: Wave[];
};

export type LogEntry = {
  id: number;
  kind: string;
  incoming: boolean;
  text: string;
  at: string;
  slack_id: string | null;
  user_name: string | null;
  admin_name: string | null;
  recipients: number;
  sent: number;
  failed: number;
  skipped: number;
  pending: number;
  error: string | null;
};

export type Draft = {
  id: number;
  kind: string;
  text: string;
  created_at: string;
};

export type Thread = {
  slack_id: string;
  user_id: number | null;
  user_name: string | null;
  last_text: string;
  last_at: string;
  unread: number;
};

export type SlackMessage = {
  id: number;
  incoming: boolean;
  kind: string;
  text: string;
  at: string;
  admin_name: string | null;
};
