use repository::metadata::model::{NewMentionedPlace, NewMetadata, NewTheme};
use serde::Deserialize;

use crate::constant::PostType;

#[derive(Deserialize)]
pub struct AgentExtractorResponse {
    metadata: Metadata,
    summary_keywords: Vec<String>,
    mentioned_places: Vec<MentiondPlace>,
}

impl Into<NewMetadata> for AgentExtractorResponse {
    fn into(self) -> NewMetadata {
        NewMetadata::new(
            self.metadata.post_type.to_str(),
            self.metadata.companion,
            self.metadata.duration,
            self.metadata.budget_level,
            self.metadata.best_season,
            self.metadata.has_cost_breakdown,
            self.summary_keywords,
        )
    }
}

impl AgentExtractorResponse {
    pub fn get_themes(&self) -> &Vec<Theme> {
        &self.metadata.themes
    }
    pub fn get_metioned_places(&self) -> &Vec<MentiondPlace> {
        &self.mentioned_places
    }
}

#[derive(Deserialize)]
pub struct Metadata {
    companion: Option<String>,
    duration: Option<String>,
    budget_level: Option<String>,
    themes: Vec<Theme>,
    post_type: PostType,
    has_cost_breakdown: bool,
    best_season: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct MentiondPlace {
    name: String,
    category: String,
    context: String,
}

impl Into<NewMentionedPlace> for &MentiondPlace {
    fn into(self) -> NewMentionedPlace {
        NewMentionedPlace::new(&self.name, &self.category, Some(self.context.clone()))
    }
}

#[derive(Deserialize, Clone)]
pub struct Theme {
    name: String,
    score: i32,
}

impl Into<NewTheme> for &Theme {
    fn into(self) -> NewTheme {
        NewTheme::new(&self.name, self.score)
    }
}
