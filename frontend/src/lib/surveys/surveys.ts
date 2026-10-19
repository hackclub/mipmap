import all from "$lib/assets/surveys.json";
import type { Survey } from "./types";

export const surveys = all as Survey[];

export const find_survey = (id: string) =>
  surveys.find((survey) => String(survey.id) === id);
