mod constant;
mod types;

use std::env;

use llm::{
    LLMAPI, gemini::models::GeminiModel, traits::TextGenerationService, types::AgentTextRequest,
};
use repository::{
    agent::{AgentRepositoryImpl, model::NewAgentReport},
    article::{ArticleRepository, ArticleRepositoryImpl},
    config::DatabaseServerConfig,
    metadata::{
        MetaRepository, MetaRepositoryImpl,
        subs::{MetaMentionedPlaceRepository, MetaThemeRepository},
    },
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{constant::ReportType, types::AgentExtractorResponse};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_config = DatabaseServerConfig::from_env();
    let pool = db_config.get_pool().await;

    let article_repo = ArticleRepositoryImpl::new(pool.clone());
    let meta_repo = MetaRepositoryImpl::new(pool.clone());
    let agent_repo = AgentRepositoryImpl::new(pool.clone());

    let llm_api = LLMAPI::from_env();
    let gemini_token = env::var("GOOGLE_API_KEY").expect("Failed to load gemini key");
    let gemini_api = llm_api.authed_gemini(&gemini_token);

    let extract_prompt = include_str!("../extract_prompt.txt");

    loop {
        let articles = article_repo.find_detail_with_no_metadata().await.unwrap();

        for article in articles.iter() {
            let prompt = extract_prompt
                .replace("{title}", article.title.as_ref().unwrap())
                .replace("{content}", article.content.as_ref().unwrap());

            let request = AgentTextRequest::new("", &prompt, false);
            let agent_response = gemini_api
                .generate_text(GeminiModel::Gemini3FlashPreview, request)
                .await
                .unwrap();

            let extract_response: AgentExtractorResponse = serde_json::from_str(
                agent_response
                    .get_content()
                    .replace("```json", "")
                    .replace("```", "")
                    .as_str(),
            )
            .unwrap();

            // repo save
            let new_agent_report = NewAgentReport::new(
                article.id,
                ReportType::Extract.to_str(),
                agent_response.get_content(),
            );
            agent_repo.save(new_agent_report).await.ok();

            let themes = extract_response.get_themes().clone();
            let places = extract_response.get_metioned_places().clone();

            // metadata save
            if let Ok(meta_id) = meta_repo
                .save(
                    article.id,
                    article.title.as_ref().unwrap().as_str(),
                    extract_response.into(),
                )
                .await
            {
                meta_repo
                    .save_themes(meta_id, themes.iter().map(|t| t.into()).collect())
                    .await
                    .ok();
                meta_repo
                    .save_places(meta_id, places.iter().map(|p| p.into()).collect())
                    .await
                    .ok();
            };
        }

        if articles.len() < 20 {
            break;
        }
    }

    println!("Hello, world!");
}
