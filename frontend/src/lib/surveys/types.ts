export type Segment = { text: string; colour?: string };

export type ScaleQuestion = {
  type: "scale";
  key: string;
  low: string;
  high: string;
  items: { key: string; label: string }[];
};

export type TextQuestion = {
  type: "text";
  key: string;
  label: string;
  optional?: boolean;
};

export type Question = ScaleQuestion | TextQuestion;

export type Survey = {
  id: number;
  title: string;
  heading: Segment[];
  questions: Question[];
};

export type Rating = number | "unsure";
