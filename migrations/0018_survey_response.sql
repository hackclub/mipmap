CREATE TABLE survey_response (
    id SERIAL PRIMARY KEY,
    survey_id INTEGER NOT NULL,
    answers JSONB NOT NULL,
    created_on DATE NOT NULL DEFAULT CURRENT_DATE
);

CREATE INDEX survey_response_survey_id ON survey_response (survey_id);
