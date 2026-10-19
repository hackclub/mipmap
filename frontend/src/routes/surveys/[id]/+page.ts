import { error } from "@sveltejs/kit";
import { find_survey, surveys } from "$lib/surveys/surveys";

export const entries = () =>
  surveys.map((survey) => ({ id: String(survey.id) }));

export const load = ({ params }) => {
  const survey = find_survey(params.id);
  if (!survey) error(404, "Survey not found");
  return { survey };
};
