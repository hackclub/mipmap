export interface Snapshot {
  id: number;
  image: string;
  caption: string;
  author: string;
}

export interface SnapshotStatus {
  tracked_seconds: number;
  required_seconds: number;
  sent_today: boolean;
}
